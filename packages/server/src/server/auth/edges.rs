use crate::server::auth::{hash_phone_number, Session, SessionStore, SessionToken};
use anyhow::Result;
use juniper::FieldResult;
use sqlx::PgPool;
use twilio::TwilioService;
use uuid::Uuid;

/// Send OTP verification code via SMS
pub async fn send_verification_code(
    phone_number: String,
    twilio: &TwilioService,
    db_pool: &PgPool,
) -> FieldResult<bool> {
    // Validate phone number format (basic validation)
    if !phone_number.starts_with('+') {
        return Err(juniper::FieldError::new(
            "Phone number must include country code (e.g., +1234567890)",
            juniper::Value::null(),
        ));
    }

    // Check if phone number exists in identifiers table
    let phone_hash = hash_phone_number(&phone_number);
    let exists = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM identifiers WHERE phone_hash = $1) as exists",
        phone_hash
    )
    .fetch_one(db_pool)
    .await
    .map_err(|e| juniper::FieldError::new(format!("Database error: {}", e), juniper::Value::null()))?
    .exists
    .unwrap_or(false);

    if !exists {
        return Err(juniper::FieldError::new(
            "Phone number not registered",
            juniper::Value::null(),
        ));
    }

    // Send OTP via Twilio
    twilio
        .send_otp(&phone_number)
        .await
        .map_err(|e| juniper::FieldError::new(format!("Failed to send OTP: {}", e), juniper::Value::null()))?;

    Ok(true)
}

/// Verify OTP code and create session
pub async fn verify_code(
    phone_number: String,
    code: String,
    twilio: &TwilioService,
    session_store: &SessionStore,
    db_pool: &PgPool,
) -> FieldResult<SessionToken> {
    // Verify OTP with Twilio
    twilio
        .verify_otp(&phone_number, &code)
        .await
        .map_err(|e| juniper::FieldError::new(format!("Invalid or expired code: {}", e), juniper::Value::null()))?;

    // Look up member by phone hash
    let phone_hash = hash_phone_number(&phone_number);
    let identifier = sqlx::query!(
        r#"
        SELECT member_id, is_admin
        FROM identifiers
        WHERE phone_hash = $1
        "#,
        phone_hash
    )
    .fetch_one(db_pool)
    .await
    .map_err(|e| juniper::FieldError::new(format!("Database error: {}", e), juniper::Value::null()))?;

    // Create session
    let session = Session {
        member_id: identifier.member_id,
        phone_number: phone_number.clone(),
        is_admin: identifier.is_admin,
        created_at: chrono::Utc::now(),
    };

    let token = session_store.create_session(session).await;

    Ok(token)
}

/// Logout (delete session)
pub async fn logout(
    session_token: String,
    session_store: &SessionStore,
) -> FieldResult<bool> {
    session_store
        .delete_session(&session_token)
        .await
        .map_err(|e| juniper::FieldError::new(format!("Failed to logout: {}", e), juniper::Value::null()))?;

    Ok(true)
}

/// Get or create member for phone number (for admin use - registering new users)
pub async fn register_phone_number(
    phone_number: String,
    is_admin: bool,
    db_pool: &PgPool,
) -> Result<Uuid> {
    let phone_hash = hash_phone_number(&phone_number);

    // Check if identifier already exists
    let existing = sqlx::query!(
        "SELECT member_id FROM identifiers WHERE phone_hash = $1",
        phone_hash
    )
    .fetch_optional(db_pool)
    .await?;

    if let Some(row) = existing {
        return Ok(row.member_id);
    }

    // Create new member
    let member_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO members (id) VALUES ($1)",
        member_id
    )
    .execute(db_pool)
    .await?;

    // Create identifier
    sqlx::query!(
        "INSERT INTO identifiers (member_id, phone_hash, is_admin) VALUES ($1, $2, $3)",
        member_id,
        phone_hash,
        is_admin
    )
    .execute(db_pool)
    .await?;

    Ok(member_id)
}
