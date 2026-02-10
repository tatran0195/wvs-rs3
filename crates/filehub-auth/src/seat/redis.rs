//! Redis-based seat allocator using Lua scripts for atomicity.
//!
//! Suitable for multi-node deployments.

#[cfg(feature = "redis-seat")]
mod implementation {
    use async_trait::async_trait;
    use redis::AsyncCommands;
    use tracing::{error, info, warn};

    use filehub_core::error::AppError;

    use crate::seat::allocator::{AllocationResult, PoolState, SeatAllocator};

    /// Redis key for the set of allocated user keys.
    const SEAT_SET_KEY: &str = "filehub:seats:allocated";
    /// Redis key for total seat count.
    const SEAT_TOTAL_KEY: &str = "filehub:seats:total";
    /// Redis key for admin reserved count.
    const SEAT_RESERVED_KEY: &str = "filehub:seats:admin_reserved";

    /// Lua script for atomic seat allocation.
    ///
    /// KEYS[1] = allocated set
    /// KEYS[2] = total key
    /// KEYS[3] = reserved key
    /// ARGV[1] = user_key
    /// ARGV[2] = is_admin ("1" or "0")
    ///
    /// Returns:
    ///   1 = granted
    ///   0 = denied (no seats)
    ///  -1 = already allocated (idempotent)
    const ALLOCATE_SCRIPT: &str = r#"
        local allocated_key = KEYS[1]
        local total_key = KEYS[2]
        local reserved_key = KEYS[3]
        local user_key = ARGV[1]
        local is_admin = tonumber(ARGV[2])

        -- Check if already allocated
        if redis.call('SISMEMBER', allocated_key, user_key) == 1 then
            return -1
        end

        local total = tonumber(redis.call('GET', total_key) or '0')
        local reserved = tonumber(redis.call('GET', reserved_key) or '0')
        local checked_out = redis.call('SCARD', allocated_key)

        local available
        if is_admin == 1 then
            available = total - checked_out
        else
            available = total - checked_out - reserved
        end

        if available <= 0 then
            if is_admin == 1 and (total - checked_out) > 0 then
                -- Admin using reserved seat
                redis.call('SADD', allocated_key, user_key)
                return 1
            end
            return 0
        end

        redis.call('SADD', allocated_key, user_key)
        return 1
    "#;

    /// Lua script for atomic seat release.
    const RELEASE_SCRIPT: &str = r#"
        local allocated_key = KEYS[1]
        local user_key = ARGV[1]
        return redis.call('SREM', allocated_key, user_key)
    "#;

    /// Redis-based seat allocator for multi-node deployments.
    #[derive(Debug, Clone)]
    pub struct RedisSeatAllocator {
        /// Redis connection manager.
        pool: redis::aio::ConnectionManager,
    }

    impl RedisSeatAllocator {
        /// Creates a new Redis-based seat allocator.
        pub async fn new(
            redis_url: &str,
            total_seats: u32,
            admin_reserved: u32,
        ) -> Result<Self, AppError> {
            let client = redis::Client::open(redis_url)
                .map_err(|e| AppError::internal(format!("Redis connection failed: {e}")))?;

            let mut conn = client
                .get_connection_manager()
                .await
                .map_err(|e| AppError::internal(format!("Redis connection manager failed: {e}")))?;

            // Initialize total seats and reserved count
            let _: () = conn
                .set(SEAT_TOTAL_KEY, total_seats)
                .await
                .map_err(|e| AppError::internal(format!("Redis SET failed: {e}")))?;

            let _: () = conn
                .set(SEAT_RESERVED_KEY, admin_reserved)
                .await
                .map_err(|e| AppError::internal(format!("Redis SET failed: {e}")))?;

            info!(
                total_seats = total_seats,
                admin_reserved = admin_reserved,
                "Redis seat allocator initialized"
            );

            Ok(Self { pool: conn })
        }
    }

    #[async_trait]
    impl SeatAllocator for RedisSeatAllocator {
        async fn try_allocate(
            &self,
            user_key: &str,
            role: &str,
        ) -> Result<AllocationResult, AppError> {
            let is_admin = if role == "admin" || role == "Admin" {
                "1"
            } else {
                "0"
            };

            let mut conn = self.pool.clone();

            let result: i64 = redis::Script::new(ALLOCATE_SCRIPT)
                .key(SEAT_SET_KEY)
                .key(SEAT_TOTAL_KEY)
                .key(SEAT_RESERVED_KEY)
                .arg(user_key)
                .arg(is_admin)
                .invoke_async(&mut conn)
                .await
                .map_err(|e| AppError::internal(format!("Redis Lua script failed: {e}")))?;

            match result {
                1 => {
                    info!(user_key = %user_key, "Seat allocated via Redis");
                    Ok(AllocationResult::Granted)
                }
                -1 => {
                    info!(user_key = %user_key, "Seat already allocated (idempotent)");
                    Ok(AllocationResult::Granted)
                }
                0 => {
                    warn!(user_key = %user_key, "Seat allocation denied: no available seats");
                    Ok(AllocationResult::Denied {
                        reason: "All available seats are occupied".to_string(),
                    })
                }
                other => {
                    error!(result = other, "Unexpected Lua script result");
                    Err(AppError::internal(format!(
                        "Unexpected seat allocation result: {other}"
                    )))
                }
            }
        }

        async fn release(&self, user_key: &str) -> Result<(), AppError> {
            let mut conn = self.pool.clone();

            let removed: i64 = redis::Script::new(RELEASE_SCRIPT)
                .key(SEAT_SET_KEY)
                .arg(user_key)
                .invoke_async(&mut conn)
                .await
                .map_err(|e| AppError::internal(format!("Redis Lua release failed: {e}")))?;

            if removed > 0 {
                info!(user_key = %user_key, "Seat released via Redis");
            } else {
                warn!(user_key = %user_key, "Seat release: key was not in allocated set");
            }

            Ok(())
        }

        async fn pool_state(&self) -> Result<PoolState, AppError> {
            let mut conn = self.pool.clone();

            let total: u32 = conn.get(SEAT_TOTAL_KEY).await.unwrap_or(0);

            let reserved: u32 = conn.get(SEAT_RESERVED_KEY).await.unwrap_or(0);

            let checked_out: u32 = conn.scard(SEAT_SET_KEY).await.unwrap_or(0);

            Ok(PoolState {
                total_seats: total,
                checked_out,
                available: total.saturating_sub(checked_out),
                admin_reserved: reserved,
                active_sessions: checked_out,
            })
        }

        async fn set_total_seats(&self, total: u32) -> Result<(), AppError> {
            let mut conn = self.pool.clone();
            let _: () = conn
                .set(SEAT_TOTAL_KEY, total)
                .await
                .map_err(|e| AppError::internal(format!("Redis SET failed: {e}")))?;
            info!(total = total, "Redis total seats updated");
            Ok(())
        }

        async fn set_admin_reserved(&self, count: u32) -> Result<(), AppError> {
            let mut conn = self.pool.clone();
            let _: () = conn
                .set(SEAT_RESERVED_KEY, count)
                .await
                .map_err(|e| AppError::internal(format!("Redis SET failed: {e}")))?;
            info!(count = count, "Redis admin reserved updated");
            Ok(())
        }

        async fn reconcile(&self, actual_active_sessions: u32) -> Result<(), AppError> {
            let state = self.pool_state().await?;

            if state.checked_out != actual_active_sessions {
                warn!(
                    pool = state.checked_out,
                    db = actual_active_sessions,
                    "Drift detected, clearing Redis seat set for reconciliation"
                );

                let mut conn = self.pool.clone();
                let _: () = conn
                    .del(SEAT_SET_KEY)
                    .await
                    .map_err(|e| AppError::internal(format!("Redis DEL failed: {e}")))?;
            }

            Ok(())
        }
    }
}

#[cfg(feature = "redis-seat")]
pub use implementation::RedisSeatAllocator;
