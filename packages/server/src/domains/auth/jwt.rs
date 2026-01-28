use anyhow::Result;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims - data stored in the token
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,          // Subject (member_id as string)
    pub member_id: Uuid,      // Member UUID
    pub phone_number: String, // Phone number (for logging/debugging)
    pub is_admin: bool,       // Admin flag
    pub exp: i64,             // Expiration timestamp
    pub iat: i64,             // Issued at timestamp
    pub iss: String,          // Issuer
    pub jti: String,          // JWT ID (unique token identifier)
}

/// JWT Service - creates and verifies JWT tokens
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
}

impl JwtService {
    /// Create new JWT service with secret and issuer
    pub fn new(secret: &str, issuer: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            issuer,
        }
    }

    /// Create a new JWT token for a member
    ///
    /// Token expires after 24 hours
    pub fn create_token(
        &self,
        member_id: Uuid,
        phone_number: String,
        is_admin: bool,
    ) -> Result<String> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::hours(24);

        let claims = Claims {
            sub: member_id.to_string(),
            member_id,
            phone_number,
            is_admin,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            iss: self.issuer.clone(),
            jti: Uuid::new_v4().to_string(), // Unique token ID
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(Into::into)
    }

    /// Verify and decode a JWT token
    ///
    /// Returns claims if token is valid and not expired
    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let service = JwtService::new("test_secret_key", "test_issuer".to_string());
        let member_id = Uuid::new_v4();

        let token = service
            .create_token(member_id, "+1234567890".to_string(), true)
            .unwrap();

        let claims = service.verify_token(&token).unwrap();
        assert_eq!(claims.member_id, member_id);
        assert_eq!(claims.phone_number, "+1234567890");
        assert_eq!(claims.is_admin, true);
        assert_eq!(claims.iss, "test_issuer");
    }

    #[test]
    fn test_invalid_token() {
        let service = JwtService::new("test_secret_key", "test_issuer".to_string());
        let result = service.verify_token("invalid_token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let service1 = JwtService::new("secret1", "test_issuer".to_string());
        let service2 = JwtService::new("secret2", "test_issuer".to_string());

        let member_id = Uuid::new_v4();
        let token = service1
            .create_token(member_id, "+1234567890".to_string(), false)
            .unwrap();

        // Token created with secret1 should not verify with secret2
        let result = service2.verify_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        // This test is conceptual - in real tests you'd use a time-mocking library
        // For now, we just verify that exp is set correctly
        let service = JwtService::new("test_secret_key", "test_issuer".to_string());
        let member_id = Uuid::new_v4();

        let token = service
            .create_token(member_id, "+1234567890".to_string(), false)
            .unwrap();

        let claims = service.verify_token(&token).unwrap();

        // Token should expire in ~24 hours
        let now = chrono::Utc::now().timestamp();
        let expires_in = claims.exp - now;
        assert!(expires_in > 23 * 3600); // At least 23 hours
        assert!(expires_in <= 24 * 3600); // At most 24 hours
    }
}
