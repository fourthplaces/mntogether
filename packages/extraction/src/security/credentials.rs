//! Credential handling with secure memory.
//!
//! Uses the `secrecy` crate to prevent accidental logging of sensitive values.

use secrecy::{ExposeSecret, SecretBox};
use std::fmt;

/// A secret string that won't be logged or displayed.
///
/// Uses `secrecy::SecretBox` to ensure API keys and other credentials
/// are never accidentally exposed in logs, debug output, or error messages.
pub struct SecretString(SecretBox<str>);

impl SecretString {
    /// Create a new secret string.
    pub fn new(value: impl Into<String>) -> Self {
        Self(SecretBox::new(Box::from(value.into().as_str())))
    }

    /// Expose the secret value for use.
    ///
    /// Only call this when actually using the secret (e.g., in an API request).
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl Clone for SecretString {
    fn clone(&self) -> Self {
        Self::new(self.expose().to_string())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Configuration for an AI service with secure credential handling.
#[derive(Clone)]
pub struct AICredentials {
    /// API key (secret)
    pub api_key: SecretString,

    /// Model identifier
    pub model: String,

    /// API base URL (optional)
    pub base_url: Option<String>,
}

impl AICredentials {
    /// Create new AI credentials.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            model: model.into(),
            base_url: None,
        }
    }

    /// Set the base URL.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }
}

impl fmt::Debug for AICredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AICredentials")
            .field("api_key", &"[REDACTED]")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_not_in_debug() {
        let secret = SecretString::new("sk-super-secret-key");
        let debug = format!("{:?}", secret);
        assert!(!debug.contains("sk-super"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn test_secret_not_in_display() {
        let secret = SecretString::new("sk-super-secret-key");
        let display = format!("{}", secret);
        assert!(!display.contains("sk-super"));
        assert!(display.contains("[REDACTED]"));
    }

    #[test]
    fn test_expose_works() {
        let secret = SecretString::new("sk-super-secret-key");
        assert_eq!(secret.expose(), "sk-super-secret-key");
    }

    #[test]
    fn test_ai_credentials_debug() {
        let creds = AICredentials::new("sk-secret", "gpt-4");
        let debug = format!("{:?}", creds);
        assert!(!debug.contains("sk-secret"));
        assert!(debug.contains("gpt-4"));
    }
}
