//! Session lifecycle manager — login, logout, refresh token flows.

use std::net::IpAddr;
use std::sync::Arc;

use chrono::Utc;
use filehub_core::config::session::OverflowStrategy;
use tracing::{error, info, warn};
use uuid::Uuid;

use filehub_cache::provider::CacheManager;
use filehub_core::config::{AuthConfig, SessionConfig};
use filehub_core::error::AppError;
use filehub_core::traits::CacheProvider;
use filehub_database::repositories::user::UserRepository;
use filehub_entity::session::Session;
use filehub_entity::user::{User, UserStatus};

use crate::jwt::encoder::TokenPair;
use crate::jwt::{Claims, JwtDecoder, JwtEncoder};
use crate::password::PasswordHasher;
use crate::seat::{AllocationResult, SeatAllocator, SessionLimiter};

use super::store::SessionStore;

/// Result of a successful login.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoginResult {
    /// Generated token pair.
    pub tokens: TokenPair,
    /// Created session.
    pub session: Session,
    /// The authenticated user.
    pub user: User,
}

/// Manages the complete session lifecycle.
#[derive(Clone)]
pub struct SessionManager {
    /// JWT encoder for token generation.
    jwt_encoder: Arc<JwtEncoder>,
    /// JWT decoder for token validation.
    jwt_decoder: Arc<JwtDecoder>,
    /// Session persistence.
    session_store: Arc<SessionStore>,
    /// User repository.
    user_repo: Arc<UserRepository>,
    /// Password hasher.
    password_hasher: Arc<PasswordHasher>,
    /// Seat allocator for concurrent session control.
    seat_allocator: Arc<dyn SeatAllocator>,
    /// Session limiter for per-user/role limits.
    session_limiter: Arc<SessionLimiter>,
    /// Cache manager.
    cache: Arc<CacheManager>,
    /// Auth configuration.
    auth_config: AuthConfig,
    /// Session configuration.
    session_config: SessionConfig,
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager")
            .field("auth_config", &self.auth_config)
            .field("session_config", &self.session_config)
            .finish()
    }
}

impl SessionManager {
    /// Creates a new session manager with all required dependencies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        jwt_encoder: Arc<JwtEncoder>,
        jwt_decoder: Arc<JwtDecoder>,
        session_store: Arc<SessionStore>,
        user_repo: Arc<UserRepository>,
        password_hasher: Arc<PasswordHasher>,
        seat_allocator: Arc<dyn SeatAllocator>,
        session_limiter: Arc<SessionLimiter>,
        cache: Arc<CacheManager>,
        auth_config: AuthConfig,
        session_config: SessionConfig,
    ) -> Self {
        Self {
            jwt_encoder,
            jwt_decoder,
            session_store,
            user_repo,
            password_hasher,
            seat_allocator,
            session_limiter,
            cache,
            auth_config,
            session_config,
        }
    }

    /// Performs the complete login flow:
    ///
    /// 1. Validate credentials
    /// 2. Check user status (active, not locked)
    /// 3. Resolve session limit for user's role
    /// 4. Check user's active session count
    /// 5. Apply overflow strategy if at limit
    /// 6. Check pool availability (admin reservation)
    /// 7. Atomic seat allocation
    /// 8. Create session + generate JWT
    /// 9. Return tokens
    ///
    /// Rolls back seat allocation on any failure after step 7.
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ip_address: IpAddr,
        user_agent: Option<&str>,
        device_info: Option<serde_json::Value>,
    ) -> Result<LoginResult, AppError> {
        // Step 1: Find user (try cache first)
        let user = if let Some(cached) = self.get_cached_user_by_name(username).await {
            cached
        } else {
            let user = self
                .user_repo
                .find_by_username(username)
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
                .ok_or_else(|| AppError::unauthorized("Invalid username or password"))?;

            // Cache for subsequent requests
            self.cache_user(&user).await;
            user
        };

        // Step 2: Check user status
        self.check_user_status(&user)?;

        // Step 3: Verify password
        let password_valid = self
            .password_hasher
            .verify_password(password, &user.password_hash)?;

        if !password_valid {
            self.handle_failed_login(&user).await?;
            // If failed, we should probably invalidate cache to force DB lookup on next try
            // in case password was changed but cache is stale (though password checking uses param vs hash)
            // But strict security might suggest invalidating.
            self.invalidate_user_cache(&user).await;
            return Err(AppError::unauthorized("Invalid username or password"));
        }

        // Reset failed attempts on successful password verification
        self.reset_failed_attempts(&user).await?;

        // Setup cache for the fresh user state (e.g. failed attempts reset)
        self.cache_user(&user).await;

        // Step 4: Resolve session limit
        let session_limit = self
            .session_limiter
            .resolve_limit(user.id, &user.role)
            .await?;

        // Step 5: Check current session count
        let active_count = self.session_store.count_active_by_user(user.id).await?;

        // Step 6: Handle overflow if at limit
        if let Some(max) = session_limit {
            if active_count >= max as i64 {
                self.handle_overflow(&user, max).await?;
            }
        }

        // Step 7: Check pool availability and allocate seat
        let allocation = self
            .seat_allocator
            .try_allocate(&user.id.to_string(), &user.role.to_string())
            .await;

        match allocation {
            Ok(AllocationResult::Granted) => {
                info!(user_id = %user.id, "Seat allocated successfully");
            }
            Ok(AllocationResult::Denied { reason }) => {
                warn!(user_id = %user.id, reason = %reason, "Seat allocation denied");
                return Err(AppError::service_unavailable(format!(
                    "Cannot login: {reason}"
                )));
            }
            Err(e) => {
                error!(user_id = %user.id, error = %e, "Seat allocation error");
                return Err(AppError::internal(format!("Seat allocation failed: {e}")));
            }
        }

        // Step 8: Create session and generate tokens
        // If anything fails from here, we must release the seat
        let result = self
            .create_session_and_tokens(&user, ip_address, user_agent, device_info)
            .await;

        match result {
            Ok(login_result) => {
                // Update last login
                let _ = self.user_repo.update_last_login(user.id).await;
                info!(
                    user_id = %user.id,
                    session_id = %login_result.session.id,
                    "Login successful"
                );
                Ok(login_result)
            }
            Err(e) => {
                // Rollback: release seat
                error!(
                    user_id = %user.id,
                    error = %e,
                    "Failed to create session, releasing seat"
                );
                let _ = self.seat_allocator.release(&user.id.to_string()).await;
                Err(e)
            }
        }
    }

    /// Performs the complete logout flow:
    ///
    /// 1. Blocklist the current JWT
    /// 2. Blocklist the session
    /// 3. Release the seat
    /// 4. Mark session as terminated
    pub async fn logout(&self, claims: &Claims) -> Result<(), AppError> {
        let session_id = claims.session_id();
        let user_id = claims.user_id();

        info!(user_id = %user_id, session_id = %session_id, "Processing logout");

        // Step 1: Blocklist the access token
        self.jwt_decoder
            .blocklist_token(claims.jti, claims.remaining_ttl_seconds())
            .await?;

        // Step 2: Blocklist the entire session (prevents refresh token usage)
        self.jwt_decoder.blocklist_session(session_id).await?;

        // Step 3: Release the seat
        if let Err(e) = self.seat_allocator.release(&user_id.to_string()).await {
            error!(
                user_id = %user_id,
                error = %e,
                "Failed to release seat during logout"
            );
        }

        // Step 4: Terminate the session in database
        self.session_store
            .terminate_session(session_id, Some(user_id), "User logout")
            .await?;

        // Invalidate session cache
        self.invalidate_session_cache(session_id).await;

        info!(user_id = %user_id, session_id = %session_id, "Logout completed");

        Ok(())
    }

    /// Refreshes an access token using a valid refresh token.
    ///
    /// 1. Validate refresh token
    /// 2. Check session is still active
    /// 3. Generate new access token
    /// 4. Optionally rotate refresh token
    pub async fn refresh(&self, refresh_token: &str) -> Result<TokenPair, AppError> {
        // Step 1: Decode refresh token
        let claims = self.jwt_decoder.decode_refresh_token(refresh_token).await?;

        // Step 2: Check session is still valid
        let session_id = claims.session_id();

        if self.jwt_decoder.is_session_blocked(&session_id).await? {
            return Err(AppError::unauthorized("Session has been terminated"));
        }

        let session = self
            .session_store
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| AppError::unauthorized("Session not found"))?;

        if session.terminated_at.is_some() {
            return Err(AppError::unauthorized("Session has been terminated"));
        }

        if session.expires_at <= Utc::now() {
            return Err(AppError::unauthorized("Session has expired"));
        }

        // Step 3: Look up current user (role may have changed)
        let user = if let Some(cached) = self.get_cached_user(claims.user_id()).await {
            cached
        } else {
            let user = self
                .user_repo
                .find_by_id(claims.user_id())
                .await
                .map_err(|e| AppError::internal(format!("Database error: {e}")))?
                .ok_or_else(|| AppError::unauthorized("User not found"))?;

            self.cache_user(&user).await;
            user
        };

        self.check_user_status(&user)?;

        // Step 4: Blocklist old refresh token
        self.jwt_decoder
            .blocklist_token(claims.jti, claims.remaining_ttl_seconds())
            .await?;

        // Step 5: Generate new token pair
        let tokens = self.jwt_encoder.generate_token_pair(
            user.id,
            session_id,
            &user.role,
            &user.username,
        )?;

        // Step 6: Update refresh token hash in session
        let new_refresh_hash = sha256_hash(&tokens.refresh_token);
        self.session_store
            .update_refresh_token(session_id, &new_refresh_hash)
            .await?;

        // Step 7: Touch activity
        self.session_store.touch_activity(session_id).await?;

        info!(
            user_id = %user.id,
            session_id = %session_id,
            "Token refreshed"
        );

        // Invalidate the session in cache because its refresh token hash and last activity changed.
        // Or we could update it, but invalidating forces a fresh fetch next time which is safer/easier.
        self.invalidate_session_cache(session_id).await;

        Ok(tokens)
    }

    /// Terminates a session by an administrator.
    pub async fn admin_terminate(
        &self,
        session_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError> {
        let session = self
            .session_store
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| AppError::not_found("Session not found"))?;

        if session.terminated_at.is_some() {
            return Err(AppError::conflict("Session is already terminated"));
        }

        info!(
            session_id = %session_id,
            admin_id = %admin_id,
            user_id = %session.user_id,
            reason = %reason,
            "Admin terminating session"
        );

        // Blocklist the session
        self.jwt_decoder.blocklist_session(session_id).await?;

        // Release seat
        if let Err(e) = self
            .seat_allocator
            .release(&session.user_id.to_string())
            .await
        {
            error!(error = %e, "Failed to release seat during admin termination");
        }

        // Terminate in database
        self.session_store
            .terminate_session(
                session_id,
                Some(admin_id),
                &format!("Admin termination: {reason}"),
            )
            .await?;

        // Invalidate session cache
        self.invalidate_session_cache(session_id).await;

        Ok(())
    }

    /// Terminates all sessions for a specific user.
    pub async fn terminate_all_user_sessions(
        &self,
        user_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<u32, AppError> {
        let sessions = self.session_store.find_active_by_user(user_id).await?;
        let mut terminated = 0u32;

        for session in &sessions {
            if let Err(e) = self.admin_terminate(session.id, admin_id, reason).await {
                error!(
                    session_id = %session.id,
                    error = %e,
                    "Failed to terminate session"
                );
            } else {
                terminated += 1;
            }
        }

        Ok(terminated)
    }

    /// Terminates all non-admin sessions.
    pub async fn terminate_all_non_admin(
        &self,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<u32, AppError> {
        let all_sessions = self.session_store.find_all_active().await?;
        let mut terminated = 0u32;

        for session in &all_sessions {
            // Skip admin sessions
            let user = self
                .user_repo
                .find_by_id(session.user_id)
                .await
                .ok()
                .flatten();

            if let Some(user) = user {
                if user.role == filehub_entity::user::UserRole::Admin {
                    continue;
                }
            }

            if let Err(e) = self.admin_terminate(session.id, admin_id, reason).await {
                error!(
                    session_id = %session.id,
                    error = %e,
                    "Failed to terminate non-admin session"
                );
            } else {
                terminated += 1;
            }
        }

        Ok(terminated)
    }

    /// Validates that the given session is still valid and active.
    pub async fn validate_session(&self, session_id: Uuid) -> Result<Session, AppError> {
        if self.jwt_decoder.is_session_blocked(&session_id).await? {
            return Err(AppError::unauthorized("Session has been blocked"));
        }

        let session = if let Some(cached) = self.get_cached_session(session_id).await {
            cached
        } else {
            let session = self
                .session_store
                .find_by_id(session_id)
                .await?
                .ok_or_else(|| AppError::unauthorized("Session not found"))?;

            self.cache_session(&session).await;
            session
        };

        if session.terminated_at.is_some() {
            return Err(AppError::unauthorized("Session has been terminated"));
        }

        if session.expires_at <= Utc::now() {
            return Err(AppError::unauthorized("Session has expired"));
        }

        // Check idle timeout
        let idle_cutoff =
            Utc::now() - chrono::Duration::minutes(self.session_config.idle_timeout_minutes as i64);

        if session.last_activity < idle_cutoff {
            // Terminate idle session
            self.session_store
                .terminate_session(session.id, None, "Idle timeout")
                .await?;

            // Release seat
            let _ = self
                .seat_allocator
                .release(&session.user_id.to_string())
                .await;

            return Err(AppError::unauthorized("Session expired due to inactivity"));
        }

        Ok(session)
    }

    /// Checks user status and lockout state.
    fn check_user_status(&self, user: &User) -> Result<(), AppError> {
        match user.status {
            UserStatus::Inactive => {
                return Err(AppError::forbidden(
                    "Account is deactivated. Contact an administrator.",
                ));
            }
            UserStatus::Locked => {
                if let Some(locked_until) = user.locked_until {
                    if locked_until > Utc::now() {
                        return Err(AppError::forbidden(format!(
                            "Account is locked until {}",
                            locked_until.format("%Y-%m-%d %H:%M:%S UTC")
                        )));
                    }
                    // Lock expired, proceed
                } else {
                    return Err(AppError::forbidden(
                        "Account is locked. Contact an administrator.",
                    ));
                }
            }
            UserStatus::Active => {}
        }
        Ok(())
    }

    /// Handles a failed login attempt by incrementing the counter and locking if needed.
    async fn handle_failed_login(&self, user: &User) -> Result<(), AppError> {
        let new_count = user.failed_login_attempts.unwrap_or(0) + 1;

        if new_count >= self.auth_config.max_failed_attempts as i32 {
            let locked_until = Utc::now()
                + chrono::Duration::minutes(self.auth_config.lockout_duration_minutes as i64);

            self.user_repo
                .lock_until(user.id, locked_until)
                .await
                .map_err(|e| AppError::internal(format!("Failed to lock user: {e}")))?;

            warn!(
                user_id = %user.id,
                username = %user.username,
                attempts = new_count,
                locked_until = %locked_until,
                "User account locked due to failed login attempts"
            );
        } else {
            let _ = self
                .user_repo
                .increment_failed_attempts(user.id)
                .await
                .map_err(|e| AppError::internal(format!("Failed to update attempts: {e}")));
        }

        Ok(())
    }

    /// Resets the failed login counter on successful authentication.
    async fn reset_failed_attempts(&self, user: &User) -> Result<(), AppError> {
        if user.failed_login_attempts.unwrap_or(0) > 0 {
            self.user_repo
                .reset_failed_attempts(user.id)
                .await
                .map_err(|e| AppError::internal(format!("Failed to reset attempts: {e}")))?;
        }
        Ok(())
    }

    /// Handles the overflow condition when a user has reached their session limit.
    async fn handle_overflow(&self, user: &User, max_sessions: u32) -> Result<(), AppError> {
        let strategy = &self.session_config.limits.overflow_strategy;

        match strategy {
            OverflowStrategy::Deny => Err(AppError::conflict(format!(
                "Maximum concurrent sessions ({max_sessions}) reached. Please log out of another session first."
            ))),
            OverflowStrategy::KickOldest => {
                let oldest = self
                    .session_store
                    .find_oldest_by_user(user.id)
                    .await?
                    .ok_or_else(|| {
                        AppError::internal("No session found to kick despite overflow")
                    })?;

                info!(
                    user_id = %user.id,
                    kicked_session = %oldest.id,
                    "Kicking oldest session due to overflow"
                );

                // Terminate the oldest session
                self.jwt_decoder.blocklist_session(oldest.id).await?;
                let _ = self.seat_allocator.release(&user.id.to_string()).await;
                self.session_store
                    .terminate_session(oldest.id, None, "Kicked: session limit overflow (oldest)")
                    .await?;

                Ok(())
            }
            OverflowStrategy::KickIdle => {
                let idle = self
                    .session_store
                    .find_most_idle_by_user(user.id)
                    .await?
                    .ok_or_else(|| {
                        AppError::internal("No session found to kick despite overflow")
                    })?;

                info!(
                    user_id = %user.id,
                    kicked_session = %idle.id,
                    last_activity = %idle.last_activity,
                    "Kicking most idle session due to overflow"
                );

                self.jwt_decoder.blocklist_session(idle.id).await?;
                let _ = self.seat_allocator.release(&user.id.to_string()).await;
                self.session_store
                    .terminate_session(idle.id, None, "Kicked: session limit overflow (most idle)")
                    .await?;

                Ok(())
            }
        }
    }

    /// Creates the session record and generates JWT tokens.
    async fn create_session_and_tokens(
        &self,
        user: &User,
        ip_address: IpAddr,
        user_agent: Option<&str>,
        device_info: Option<serde_json::Value>,
    ) -> Result<LoginResult, AppError> {
        // Generate a preliminary session ID for JWT claims
        let session_id = Uuid::new_v4();

        // Generate token pair
        let tokens = self.jwt_encoder.generate_token_pair(
            user.id,
            session_id,
            &user.role,
            &user.username,
        )?;

        // Hash tokens for storage
        let token_hash = sha256_hash(&tokens.access_token);
        let refresh_hash = sha256_hash(&tokens.refresh_token);

        // Create session record
        let mut session = self
            .session_store
            .create_session(
                user.id,
                &token_hash,
                &refresh_hash,
                ip_address,
                user_agent,
                device_info,
            )
            .await?;

        let tokens = self.jwt_encoder.generate_token_pair(
            user.id,
            session.id,
            &user.role,
            &user.username,
        )?;

        // Update session with correct hashes
        let _ = self
            .session_store
            .update_refresh_token(session.id, &refresh_hash)
            .await;

        // Mark seat as allocated
        self.session_store.set_seat_allocated(session.id).await?;

        session.seat_allocated_at = Some(Utc::now());

        Ok(LoginResult {
            tokens,
            session,
            user: user.clone(),
        })
    }
    /// Invalidates the user cache (by ID and username if possible).
    async fn invalidate_user_cache(&self, user: &User) {
        let _ = self.cache.delete(&format!("user:id:{}", user.id)).await;
        let _ = self
            .cache
            .delete(&format!("user:name:{}", user.username))
            .await;
    }

    /// Invalidates the session cache.
    async fn invalidate_session_cache(&self, session_id: Uuid) {
        let _ = self.cache.delete(&format!("session:{}", session_id)).await;
    }

    /// Caches the user for faster lookups.
    async fn cache_user(&self, user: &User) {
        // Cache by ID
        if let Ok(json) = serde_json::to_string(user) {
            let _ = self
                .cache
                .set(
                    &format!("user:id:{}", user.id),
                    &json,
                    std::time::Duration::from_secs(900), // 15 min
                )
                .await;
        }

        // Cache by Username
        if let Ok(json) = serde_json::to_string(user) {
            let _ = self
                .cache
                .set(
                    &format!("user:name:{}", user.username),
                    &json,
                    std::time::Duration::from_secs(900), // 15 min
                )
                .await;
        }
    }

    /// Caches the session for faster validation.
    async fn cache_session(&self, session: &Session) {
        if let Ok(json) = serde_json::to_string(session) {
            let _ = self
                .cache
                .set(
                    &format!("session:{}", session.id),
                    &json,
                    std::time::Duration::from_secs(300), // 5 min
                )
                .await;
        }
    }

    /// Tries to get a cached user by ID.
    async fn get_cached_user(&self, user_id: Uuid) -> Option<User> {
        if let Ok(Some(json)) = self.cache.get(&format!("user:id:{}", user_id)).await {
            if let Ok(user) = serde_json::from_str(&json) {
                return Some(user);
            }
        }
        None
    }

    /// Tries to get a cached user by Username.
    async fn get_cached_user_by_name(&self, username: &str) -> Option<User> {
        if let Ok(Some(json)) = self.cache.get(&format!("user:name:{}", username)).await {
            if let Ok(user) = serde_json::from_str(&json) {
                return Some(user);
            }
        }
        None
    }

    /// Tries to get a cached session.
    async fn get_cached_session(&self, session_id: Uuid) -> Option<Session> {
        if let Ok(Some(json)) = self.cache.get(&format!("session:{}", session_id)).await {
            if let Ok(session) = serde_json::from_str(&json) {
                return Some(session);
            }
        }
        None
    }
}

/// Computes a SHA-256 hash of the input string and returns it as a hex string.
fn sha256_hash(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
