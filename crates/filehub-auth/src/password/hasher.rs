//! Argon2id password hashing and verification.

use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher as ArgonHasher, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};

use filehub_core::error::AppError;

/// Handles password hashing and verification using Argon2id.
#[derive(Debug, Clone)]
pub struct PasswordHasher;

impl PasswordHasher {
    /// Creates a new password hasher instance.
    pub fn new() -> Self {
        Self
    }

    /// Hashes a plaintext password using Argon2id with a random salt.
    pub fn hash_password(&self, password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::internal(format!("Password hashing failed: {e}")))?;

        Ok(hash.to_string())
    }

    /// Verifies a plaintext password against a stored Argon2id hash.
    ///
    /// Returns `Ok(true)` if the password matches, `Ok(false)` if not.
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AppError::internal(format!("Invalid password hash format: {e}")))?;

        let argon2 = Argon2::default();
        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AppError::internal(format!(
                "Password verification failed: {e}"
            ))),
        }
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}
