# Admin Identifier Whitelisting Guide

## Overview

The admin authentication system uses an identifier whitelist to grant admin privileges. Both emails and phone numbers are supported. The system is already set up with the infrastructure, but requires identifier creation in the database.

## Configuration

### Environment Variable

Set the `ADMIN_IDENTIFIERS` environment variable with a comma-separated list of admin emails and/or phone numbers:

```bash
# .env or environment
ADMIN_IDENTIFIERS=admin@example.com,+15551234567
```

### How It Works

1. **Identifier Whitelisting** (`src/domains/auth/models/identifier.rs`)
   - The `is_admin_identifier()` function checks if an identifier is in the whitelist
   - Case-insensitive matching for emails
   - Exact matching for phone numbers (E.164 format)

2. **Admin Flag Storage**
   - Each identifier has an `is_admin` boolean field
   - Stored in the database during identifier creation

3. **JWT Token** (`src/domains/auth/jwt.rs`)
   - The `is_admin` flag is included in the JWT claims
   - Server validates this on every request

4. **Authorization Checks**
   - `ctx.require_admin()` enforces admin-only mutations
   - Returns `Unauthorized: Admin access required` error if not admin

## Admin Identifier Creation

### Using Test Data

The simplest way: restore the test database which includes a pre-configured admin:

```bash
docker compose exec -T postgres psql -U postgres -d rooteditorial < data/local_test_db.sql
```

This creates an admin user with phone `+1234567890` (use with `TEST_IDENTIFIER_ENABLED=true`).

### Manual Creation

```sql
-- 1. Create a member record
INSERT INTO members (expo_push_token, searchable_text, active)
VALUES ('admin:token', 'Admin user', true);

-- 2. Create an identifier with the hash of the phone/email
-- For phone +1234567890:
INSERT INTO identifiers (member_id, phone_hash, is_admin)
SELECT
    id,
    encode(sha256('+1234567890'::bytea), 'hex'),
    true
FROM members WHERE expo_push_token = 'admin:token';
```

## Security Notes

1. **JWT Expiration** - Tokens expire after 24 hours
2. **Secret Rotation** - Change `JWT_SECRET` to invalidate all tokens
3. **HTTPS Only** - Always use HTTPS in production
4. **OTP Verification** - Twilio verifies identifier ownership via OTP
5. **Case Insensitive** - Email matching is case-insensitive

## Testing

### Development Mode

Enable test identifier for easier testing:

```bash
TEST_IDENTIFIER_ENABLED=true
```

Then use `+1234567890` with any verification code.

**WARNING**: Never enable `TEST_IDENTIFIER_ENABLED` in production!
