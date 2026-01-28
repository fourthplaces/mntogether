use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Session token (random UUID)
pub type SessionToken = String;

/// Session data stored after successful OTP verification
#[derive(Clone, Debug)]
pub struct Session {
    pub member_id: Uuid,
    pub phone_number: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// In-memory session store
///
/// Sessions expire after 24 hours
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<SessionToken, Session>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session and return the token
    pub async fn create_session(&self, session: Session) -> SessionToken {
        let token = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.write().await;
        sessions.insert(token.clone(), session);
        token
    }

    /// Get session by token
    pub async fn get_session(&self, token: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(token)?;

        // Check if session is expired (24 hours)
        let now = chrono::Utc::now();
        let elapsed = now.signed_duration_since(session.created_at);
        if elapsed.num_hours() >= 24 {
            // Session expired
            return None;
        }

        Some(session.clone())
    }

    /// Delete session (logout)
    pub async fn delete_session(&self, token: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(token);
        Ok(())
    }

    /// Clean up expired sessions (run periodically)
    pub async fn cleanup_expired(&self) {
        let mut sessions = self.sessions.write().await;
        let now = chrono::Utc::now();

        sessions.retain(|_, session| {
            let elapsed = now.signed_duration_since(session.created_at);
            elapsed.num_hours() < 24
        });
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a phone number for storage in identifiers table
pub fn hash_phone_number(phone_number: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(phone_number.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let store = SessionStore::new();
        let session = Session {
            member_id: Uuid::new_v4(),
            phone_number: "+1234567890".to_string(),
            is_admin: true,
            created_at: chrono::Utc::now(),
        };

        let token = store.create_session(session.clone()).await;
        assert!(!token.is_empty());

        let retrieved = store.get_session(&token).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().phone_number, session.phone_number);
    }

    #[tokio::test]
    async fn test_session_expiration() {
        let store = SessionStore::new();
        let session = Session {
            member_id: Uuid::new_v4(),
            phone_number: "+1234567890".to_string(),
            is_admin: true,
            created_at: chrono::Utc::now() - chrono::Duration::hours(25),
        };

        let token = store.create_session(session).await;
        let retrieved = store.get_session(&token).await;
        assert!(retrieved.is_none(), "Expired session should return None");
    }

    #[test]
    fn test_phone_hash() {
        let hash1 = hash_phone_number("+1234567890");
        let hash2 = hash_phone_number("+1234567890");
        assert_eq!(hash1, hash2, "Same phone should produce same hash");

        let hash3 = hash_phone_number("+9876543210");
        assert_ne!(hash1, hash3, "Different phones should have different hashes");
    }
}
