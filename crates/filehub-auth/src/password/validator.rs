//! Password policy enforcement for new passwords.

use filehub_core::config::AuthConfig;
use filehub_core::error::AppError;

/// Validates password strength against configured policies.
#[derive(Debug, Clone)]
pub struct PasswordValidator {
    /// Minimum password length.
    min_length: usize,
}

impl PasswordValidator {
    /// Creates a new validator from auth configuration.
    pub fn new(config: &AuthConfig) -> Self {
        Self {
            min_length: config.password_min_length as usize,
        }
    }

    /// Validates a password against all configured policies.
    ///
    /// Returns `Ok(())` if the password meets all requirements,
    /// or an error describing the first violation found.
    pub fn validate(&self, password: &str) -> Result<(), AppError> {
        if password.len() < self.min_length {
            return Err(AppError::validation(format!(
                "Password must be at least {} characters long",
                self.min_length
            )));
        }

        if !password.chars().any(|c| c.is_uppercase()) {
            return Err(AppError::validation(
                "Password must contain at least one uppercase letter",
            ));
        }

        if !password.chars().any(|c| c.is_lowercase()) {
            return Err(AppError::validation(
                "Password must contain at least one lowercase letter",
            ));
        }

        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(AppError::validation(
                "Password must contain at least one digit",
            ));
        }

        if !password.chars().any(|c| !c.is_alphanumeric()) {
            return Err(AppError::validation(
                "Password must contain at least one special character",
            ));
        }

        // Use zxcvbn for entropy check
        let estimate = zxcvbn::zxcvbn(password, &[]);
        if estimate.score() < zxcvbn::Score::Three {
            return Err(AppError::validation(
                "Password is too weak. Please use a stronger password with more entropy.",
            ));
        }

        Ok(())
    }

    /// Validates that a new password differs from the old one.
    pub fn validate_not_same(
        &self,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        if old_password == new_password {
            return Err(AppError::validation(
                "New password must be different from the current password",
            ));
        }
        Ok(())
    }
}
