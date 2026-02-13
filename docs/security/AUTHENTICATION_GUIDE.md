# Authentication System Guide

## Overview

The authentication system supports **both phone numbers and email addresses** as identifiers. Twilio handles OTP delivery to either.

## Environment Variables

### Required for Production
```bash
# JWT Configuration
JWT_SECRET=your-secret-key
JWT_ISSUER=mndigitalaid

# Twilio Configuration (supports phone and email)
TWILIO_ACCOUNT_SID=your-account-sid
TWILIO_AUTH_TOKEN=your-auth-token
TWILIO_VERIFY_SERVICE_SID=your-service-sid

# Admin Configuration
ADMIN_EMAILS=admin@example.com,admin2@example.com
```

### Development/Testing Only
```bash
# Enable test identifier bypass (NEVER enable in production)
TEST_IDENTIFIER_ENABLED=true
```

## Authentication Flow

### 1. Send OTP Code

**GraphQL Mutation:**
```graphql
mutation {
  sendVerificationCode(phoneNumber: "+1234567890")  # or "user@example.com"
}
```

**Supported Identifiers:**
- Phone numbers: Must include country code (e.g., `+1234567890`)
- Email addresses: Standard format (e.g., `user@example.com`)

### 2. Verify OTP Code

**GraphQL Mutation:**
```graphql
mutation {
  verifyCode(phoneNumber: "+1234567890", code: "123456")
}
```

**Returns:** JWT token for authenticated requests

## Test Identifiers (Development Only)

When `TEST_IDENTIFIER_ENABLED=true`:

**Test Phone Number:**
- Identifier: `+1234567890`
- OTP Code: `123456`

**Test Email:**
- Identifier: `test@example.com`
- OTP Code: `123456`

**⚠️ Security Warning:**
- Test identifiers bypass Twilio verification
- MUST be registered in database before use
- Production safety check: Logs error if enabled in release build

## Admin Configuration

Admins are identified by email addresses in `ADMIN_EMAILS` environment variable:

```bash
ADMIN_EMAILS=admin@example.com,owner@example.com,superuser@example.com
```

**Usage:** Available in effects via `ctx.deps().admin_emails`

## Security Features

✅ **Implemented:**
- JWT-based stateless authentication
- OTP verification via Twilio (phone & email)
- Phone number hashing (SHA256) for database storage
- Test bypass only via explicit environment variable
- Production safety warning if test mode enabled in release build
- Admin authorization checks in effects

⚠️ **Important:**
- Test identifier bypass should NEVER be enabled in production
- The system logs a security warning if `TEST_IDENTIFIER_ENABLED=true` in release build
- Always use environment variables for sensitive configuration

## Code Architecture

**Terminology:**
- Field names still use `phone_number` for backward compatibility
- Comments clarify that identifier can be phone or email
- Twilio API natively supports both types

**Key Files:**
- `src/domains/auth/effects.rs` - OTP sending & verification logic
- `src/domains/auth/edges/mutation.rs` - GraphQL mutations
- `src/domains/auth/models.rs` - Identifier storage (hashed)
- `src/config.rs` - Environment variable loading
- `src/domains/organization/effects/deps.rs` - Server dependencies

## Migration Notes

The typed ID migration is complete:
- All `Uuid` references replaced with typed IDs (`MemberId`, `NeedId`, `PostId`, etc.)
- Compile-time type safety throughout the codebase
- No runtime behavior changes
