use crate::domains::auth::events::AuthEvent;
use crate::server::graphql::context::GraphQLContext;
use juniper::FieldResult;
use seesaw::dispatch_request;

/// Send OTP verification code via SMS
pub async fn send_verification_code(
    phone_number: String,
    ctx: &GraphQLContext,
) -> FieldResult<bool> {
    // Validate phone number format
    if !phone_number.starts_with('+') {
        return Err(juniper::FieldError::new(
            "Phone number must include country code (e.g., +1234567890)",
            juniper::Value::null(),
        ));
    }

    // Dispatch request event and wait for result
    dispatch_request(
        AuthEvent::SendOTPRequested {
            phone_number: phone_number.clone(),
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &AuthEvent| match e {
                AuthEvent::OTPSent { .. } => Some(Ok(true)),
                AuthEvent::PhoneNotRegistered { .. } => {
                    Some(Err(anyhow::anyhow!("Phone number not registered")))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))
}

/// Verify OTP code and create JWT token
pub async fn verify_code(
    phone_number: String,
    code: String,
    ctx: &GraphQLContext,
) -> FieldResult<String> {
    // Dispatch verification request and await result
    let (member_id, is_admin) = dispatch_request(
        AuthEvent::VerifyOTPRequested {
            phone_number: phone_number.clone(),
            code,
        },
        &ctx.bus,
        |m| {
            m.try_match(|e: &AuthEvent| match e {
                AuthEvent::OTPVerified {
                    member_id,
                    is_admin,
                    ..
                } => Some(Ok((*member_id, *is_admin))),
                AuthEvent::OTPFailed { reason, .. } => {
                    Some(Err(anyhow::anyhow!("Verification failed: {}", reason)))
                }
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))?;

    // Create JWT token
    let token = ctx
        .jwt_service
        .create_token(member_id, phone_number, is_admin)
        .map_err(|e| juniper::FieldError::new(e.to_string(), juniper::Value::null()))?;

    Ok(token)
}

/// Logout (JWT - client-side only, no server state to clear)
pub async fn logout(_session_token: String, _ctx: &GraphQLContext) -> FieldResult<bool> {
    // With JWT, logout is client-side only (delete token from client storage)
    // Server has no session state to clean up
    Ok(true)
}
