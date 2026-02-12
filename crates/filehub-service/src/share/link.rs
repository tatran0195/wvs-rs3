//! Share link token generation and validation.

/// Generates and validates share link tokens.
#[derive(Debug, Clone)]
pub struct LinkService;

impl LinkService {
    /// Creates a new link service.
    pub fn new() -> Self {
        Self
    }

    /// Generates a cryptographically secure random token for share links.
    pub fn generate_token(&self) -> String {
        let bytes: Vec<u8> = (0..32).map(|_| rand::random()).collect();
        hex::encode(bytes)
    }
}

impl Default for LinkService {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple hex encoding without external dependency.
mod hex {
    /// Encode bytes to hex string.
    pub fn encode(bytes: Vec<u8>) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
