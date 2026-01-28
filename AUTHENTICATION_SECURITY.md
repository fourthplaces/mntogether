# Authentication Security Guide

## ✅ Security Features Implemented

### 1. Identifier-Based Authentication
- **Supports**: Phone numbers (e.g., `+1234567890`) and email addresses (e.g., `user@example.com`)
- **Method**: OTP (One-Time Password) via Twilio
- **Storage**: SHA-256 hashed identifiers (never store plaintext)
- **Tokens**: JWT-based stateless authentication

### 2. Production Safety Checks
```rust
// Automatic warning if test mode enabled in production
if ctx.deps().test_identifier_enabled && !cfg!(debug_assertions) {
    error!("⚠️  SECURITY WARNING: TEST_IDENTIFIER_ENABLED in production!");
}
```

### 3. Admin Authorization
- **Configuration**: `ADMIN_EMAILS` environment variable
- **Matching**: Case-insensitive email comparison
- **Helper Function**: `is_admin_identifier(identifier, admin_emails)`
- **Usage**: Available in effects via `ctx.deps().admin_emails`

### 4. Test Mode (Development Only)
- **Environment**: `TEST_IDENTIFIER_ENABLED=true`
- **Test Phone**: `+1234567890` with OTP `123456`
- **Test Email**: `test@example.com` with OTP `123456`
- **Requirement**: Identifier must exist in database
- **Safety**: Logs warning if enabled in release build

## 🔒 Security Best Practices

### Environment Variables

**Production:**
```bash
# NEVER set this in production
# TEST_IDENTIFIER_ENABLED=false  # (default)

# Required
JWT_SECRET=<strong-random-secret>
JWT_ISSUER=mndigitalaid
TWILIO_ACCOUNT_SID=<your-sid>
TWILIO_AUTH_TOKEN=<your-token>
TWILIO_VERIFY_SERVICE_SID=<your-service-sid>

# Admin emails (comma-separated)
ADMIN_EMAILS=admin@example.com,owner@example.com
```

**Development:**
```bash
# Enable test identifiers for local dev
TEST_IDENTIFIER_ENABLED=true

# Same as production (use dev credentials)
JWT_SECRET=dev-secret
TWILIO_ACCOUNT_SID=<dev-sid>
TWILIO_AUTH_TOKEN=<dev-token>
TWILIO_VERIFY_SERVICE_SID=<dev-service-sid>
ADMIN_EMAILS=test@example.com
```

### Admin Email Management

**Adding Admins:**
1. Add email to `ADMIN_EMAILS` environment variable
2. User registers with that email
3. System automatically grants admin privileges

**Security Notes:**
- Emails are case-insensitive
- Uses `is_admin_identifier()` helper for checking
- Admin status stored in `identifiers.is_admin` column
- Phone numbers do NOT auto-become admin from email list

## 🔐 Authentication Flow

### 1. Send OTP
```graphql
mutation {
  sendVerificationCode(phoneNumber: "user@example.com")
}
```

**Backend:**
- Validates identifier format (phone or email)
- Checks if identifier exists in database (hashed lookup)
- Sends OTP via Twilio
- Returns success/failure

### 2. Verify OTP
```graphql
mutation {
  verifyCode(phoneNumber: "user@example.com", code: "123456")
}
```

**Backend:**
- Verifies OTP with Twilio (or bypass if test mode)
- Looks up member via hashed identifier
- Creates JWT token with admin status
- Returns token for authenticated requests

### 3. Authenticated Requests
```http
Authorization: Bearer <jwt-token>
```

**Backend:**
- JWT middleware extracts and verifies token
- Adds `AuthUser` to request extensions
- GraphQL resolvers check `ctx.auth_user.is_admin`

## 🛡️ Security Considerations

### What's Protected ✅
- ✅ OTP verification via Twilio
- ✅ Identifier hashing (SHA-256)
- ✅ JWT stateless auth
- ✅ Admin authorization checks
- ✅ Test mode safety warnings
- ✅ Case-insensitive email matching
- ✅ Environment-based configuration

### Potential Improvements 🔧

1. **Rate Limiting**
   - Add per-identifier OTP request limits
   - Prevent brute force attacks on test identifiers

2. **JWT Expiration**
   - Configure shorter token lifetimes
   - Implement refresh token rotation

3. **Audit Logging**
   - Log all admin actions
   - Track failed authentication attempts

4. **MFA Support**
   - Add optional second factor
   - Hardware key support

5. **Phone Admin List**
   - Add `ADMIN_PHONES` environment variable
   - Auto-admin for phone numbers (like emails)

## 📊 Database Schema

```sql
-- Identifiers table stores hashed phone numbers and emails
CREATE TABLE identifiers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    member_id UUID NOT NULL REFERENCES members(id),
    phone_hash TEXT NOT NULL UNIQUE,  -- SHA-256 of phone/email
    is_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for fast lookup by hash
CREATE INDEX idx_identifiers_phone_hash ON identifiers(phone_hash);
```

## 🧪 Testing

### Unit Tests
```bash
cargo test auth::models::identifier::tests
```

**Covers:**
- Hash consistency (same input → same hash)
- Hash uniqueness (different input → different hash)
- Email and phone hashing
- Admin identifier checking
- Case-insensitive matching

### Integration Testing
```bash
# 1. Enable test mode
export TEST_IDENTIFIER_ENABLED=true

# 2. Register test identifier
# (Manual step or via admin interface)

# 3. Test authentication
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "mutation { verifyCode(phoneNumber: \"+1234567890\", code: \"123456\") }"
  }'
```

## 📝 Code Examples

### Check Admin Status
```rust
// In effect or edge
if !ctx.auth_user.is_admin {
    return Err(FieldError::new(
        "Admin access required",
        juniper::Value::null(),
    ));
}
```

### Determine Admin Status at Registration
```rust
use crate::domains::auth::models::is_admin_identifier;

let is_admin = is_admin_identifier(&identifier, &admin_emails);
```

### Hash Identifier
```rust
use crate::domains::auth::models::hash_phone_number;

// Works for both phones and emails
let phone_hash = hash_phone_number("+1234567890");
let email_hash = hash_phone_number("user@example.com");
```

## 🚨 Security Checklist

Before deploying to production:

- [ ] `TEST_IDENTIFIER_ENABLED` is NOT set (or explicitly `false`)
- [ ] `JWT_SECRET` is a strong random value (32+ characters)
- [ ] `ADMIN_EMAILS` contains only verified admin emails
- [ ] Twilio credentials are production (not test)
- [ ] Database backups enabled for `identifiers` table
- [ ] Monitor logs for "SECURITY WARNING" messages
- [ ] Rate limiting enabled on `/graphql` endpoint
- [ ] HTTPS enforced (no HTTP in production)
- [ ] CORS properly configured (`ALLOWED_ORIGINS`)

## 📚 Related Files

**Authentication Core:**
- `src/domains/auth/effects.rs` - OTP sending & verification
- `src/domains/auth/edges/mutation.rs` - GraphQL mutations
- `src/domains/auth/models/identifier.rs` - Identifier model & helpers

**Configuration:**
- `src/config.rs` - Environment variable loading
- `src/domains/organization/effects/deps.rs` - Server dependencies

**Middleware:**
- `src/server/middleware/jwt_auth.rs` - JWT authentication
- `src/server/app.rs` - Application setup

**Documentation:**
- `AUTHENTICATION_GUIDE.md` - User-facing setup guide
- `AUTHENTICATION_SECURITY.md` - This file
