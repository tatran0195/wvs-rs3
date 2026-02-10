//! User repository implementation.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use filehub_core::error::{AppError, ErrorKind};
use filehub_core::result::AppResult;
use filehub_core::types::pagination::{PageRequest, PageResponse};
use filehub_entity::user::model::{CreateUser, UpdateUser};
use filehub_entity::user::{User, UserRole, UserStatus};

/// Repository for user CRUD and query operations.
#[derive(Debug, Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    /// Create a new user repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find a user by primary key.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to find user by id", e))
    }

    /// Find a user by username (case-insensitive).
    pub async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE LOWER(username) = LOWER($1)")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to find user by username", e)
            })
    }

    /// Find a user by email (case-insensitive).
    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE LOWER(email) = LOWER($1)")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to find user by email", e)
            })
    }

    /// List all users with pagination.
    pub async fn find_all(&self, page: &PageRequest) -> AppResult<PageResponse<User>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count users", e))?;

        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to list users", e))?;

        Ok(PageResponse::new(
            users,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// List users filtered by role.
    pub async fn find_by_role(
        &self,
        role: UserRole,
        page: &PageRequest,
    ) -> AppResult<PageResponse<User>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = $1")
            .bind(&role)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to count users by role", e)
            })?;

        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE role = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(&role)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to list users by role", e)
        })?;

        Ok(PageResponse::new(
            users,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// List users filtered by status.
    pub async fn find_by_status(
        &self,
        status: UserStatus,
        page: &PageRequest,
    ) -> AppResult<PageResponse<User>> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE status = $1")
            .bind(&status)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to count users by status", e)
            })?;

        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(&status)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(ErrorKind::Database, "Failed to list users by status", e)
        })?;

        Ok(PageResponse::new(
            users,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Search users by username or display name.
    pub async fn search(&self, query: &str, page: &PageRequest) -> AppResult<PageResponse<User>> {
        let pattern = format!("%{query}%");

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE username ILIKE $1 OR display_name ILIKE $1 OR email ILIKE $1"
        )
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count search results", e))?;

        let users = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username ILIKE $1 OR display_name ILIKE $1 OR email ILIKE $1 \
             ORDER BY username ASC LIMIT $2 OFFSET $3"
        )
            .bind(&pattern)
            .bind(page.limit() as i64)
            .bind(page.offset() as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to search users", e))?;

        Ok(PageResponse::new(
            users,
            page.page,
            page.page_size,
            total as u64,
        ))
    }

    /// Create a new user.
    pub async fn create(&self, data: &CreateUser) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash, display_name, role, created_by) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING *",
        )
        .bind(&data.username)
        .bind(&data.email)
        .bind(&data.password_hash)
        .bind(&data.display_name)
        .bind(&data.role)
        .bind(data.created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("users_username_key") =>
            {
                AppError::conflict(format!("Username '{}' already exists", data.username))
            }
            sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("users_email_key") => {
                AppError::conflict("Email already in use".to_string())
            }
            _ => AppError::with_source(ErrorKind::Database, "Failed to create user", e),
        })
    }

    /// Update a user's profile fields.
    pub async fn update(&self, data: &UpdateUser) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            "UPDATE users SET email = COALESCE($2, email), \
                              display_name = COALESCE($3, display_name), \
                              updated_at = NOW() \
             WHERE id = $1 RETURNING *",
        )
        .bind(data.id)
        .bind(&data.email)
        .bind(&data.display_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to update user", e))?
        .ok_or_else(|| AppError::not_found(format!("User {} not found", data.id)))
    }

    /// Update a user's password hash.
    pub async fn update_password(&self, user_id: Uuid, password_hash: &str) -> AppResult<()> {
        let result =
            sqlx::query("UPDATE users SET password_hash = $2, updated_at = NOW() WHERE id = $1")
                .bind(user_id)
                .bind(password_hash)
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    AppError::with_source(ErrorKind::Database, "Failed to update password", e)
                })?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found(format!("User {user_id} not found")));
        }
        Ok(())
    }

    /// Update a user's role.
    pub async fn update_role(&self, user_id: Uuid, role: UserRole) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            "UPDATE users SET role = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(user_id)
        .bind(&role)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to update role", e))?
        .ok_or_else(|| AppError::not_found(format!("User {user_id} not found")))
    }

    /// Update a user's status.
    pub async fn update_status(&self, user_id: Uuid, status: UserStatus) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            "UPDATE users SET status = $2, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(user_id)
        .bind(&status)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to update status", e))?
        .ok_or_else(|| AppError::not_found(format!("User {user_id} not found")))
    }

    /// Increment failed login attempts.
    pub async fn increment_failed_attempts(&self, user_id: Uuid) -> AppResult<i32> {
        let row: (i32,) = sqlx::query_as(
            "UPDATE users SET failed_login_attempts = COALESCE(failed_login_attempts, 0) + 1, \
                              updated_at = NOW() \
             WHERE id = $1 RETURNING failed_login_attempts",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorKind::Database,
                "Failed to increment failed attempts",
                e,
            )
        })?;

        Ok(row.0)
    }

    /// Reset failed login attempts to zero.
    pub async fn reset_failed_attempts(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, updated_at = NOW() WHERE id = $1"
        )
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to reset failed attempts", e))?;
        Ok(())
    }

    /// Lock a user account until the given time.
    pub async fn lock_until(&self, user_id: Uuid, until: DateTime<Utc>) -> AppResult<()> {
        sqlx::query(
            "UPDATE users SET status = 'locked', locked_until = $2, updated_at = NOW() WHERE id = $1"
        )
            .bind(user_id)
            .bind(until)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to lock user", e))?;
        Ok(())
    }

    /// Update last login timestamp.
    pub async fn update_last_login(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE users SET last_login_at = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AppError::with_source(ErrorKind::Database, "Failed to update last login", e)
            })?;
        Ok(())
    }

    /// Delete a user by ID.
    pub async fn delete(&self, user_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to delete user", e))?;

        Ok(result.rows_affected() > 0)
    }

    /// Count total users.
    pub async fn count(&self) -> AppResult<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::with_source(ErrorKind::Database, "Failed to count users", e))?;
        Ok(count as u64)
    }
}
