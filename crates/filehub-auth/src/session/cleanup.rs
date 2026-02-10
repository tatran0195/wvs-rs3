//! Expired and idle session cleanup.

use std::sync::Arc;

use tracing::{error, info};

use filehub_core::error::AppError;

use crate::jwt::JwtDecoder;
use crate::seat::SeatAllocator;

use super::store::SessionStore;

/// Handles periodic cleanup of expired and idle sessions.
#[derive(Clone)]
pub struct SessionCleanup {
    /// Session store for querying and terminating sessions.
    session_store: Arc<SessionStore>,
    /// JWT decoder for session blocklisting.
    jwt_decoder: Arc<JwtDecoder>,
    /// Seat allocator for releasing seats.
    seat_allocator: Arc<dyn SeatAllocator>,
}

impl std::fmt::Debug for SessionCleanup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionCleanup").finish()
    }
}

impl SessionCleanup {
    /// Creates a new session cleanup handler.
    pub fn new(
        session_store: Arc<SessionStore>,
        jwt_decoder: Arc<JwtDecoder>,
        seat_allocator: Arc<dyn SeatAllocator>,
    ) -> Self {
        Self {
            session_store,
            jwt_decoder,
            seat_allocator,
        }
    }

    /// Runs a cleanup cycle, terminating all expired and idle sessions.
    ///
    /// Returns the number of sessions cleaned up.
    pub async fn run_cleanup(&self) -> Result<u32, AppError> {
        let expired = self.session_store.find_expired_sessions().await?;

        if expired.is_empty() {
            return Ok(0);
        }

        info!(
            count = expired.len(),
            "Found expired/idle sessions to clean up"
        );

        let mut cleaned = 0u32;

        for session in &expired {
            // Blocklist the session's tokens
            if let Err(e) = self.jwt_decoder.blocklist_session(session.id).await {
                error!(
                    session_id = %session.id,
                    error = %e,
                    "Failed to blocklist expired session"
                );
                continue;
            }

            // Release the seat
            if let Err(e) = self
                .seat_allocator
                .release(&session.user_id.to_string())
                .await
            {
                error!(
                    session_id = %session.id,
                    error = %e,
                    "Failed to release seat for expired session"
                );
            }

            // Terminate in database
            let reason = if session.expires_at <= chrono::Utc::now() {
                "Absolute timeout expired"
            } else {
                "Idle timeout expired"
            };

            if let Err(e) = self
                .session_store
                .terminate_session(session.id, None, reason)
                .await
            {
                error!(
                    session_id = %session.id,
                    error = %e,
                    "Failed to terminate expired session"
                );
                continue;
            }

            cleaned += 1;
        }

        info!(cleaned = cleaned, "Session cleanup completed");

        Ok(cleaned)
    }
}
