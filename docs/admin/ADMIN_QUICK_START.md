# Admin Authentication Quick Start

## Overview

Admin identifiers (emails or phone numbers) are whitelisted via the `ADMIN_IDENTIFIERS` environment variable. When a whitelisted identifier verifies via OTP, they receive an admin JWT token.

## Quick Setup

### 1. Configure Admin Identifiers

Add to `.env`:

```bash
# Admin identifiers (comma-separated — emails or phone numbers)
ADMIN_IDENTIFIERS=admin@example.com,+15551234567

# Development testing (NEVER in production!)
TEST_IDENTIFIER_ENABLED=true
```

### 2. Create Admin in Database

Admins must have a member + identifier record in the database. For local dev, the test data (`data/local_test_db.sql`) includes a pre-configured admin user.

To create manually:

```sql
-- Create a member for the admin
INSERT INTO members (expo_push_token, searchable_text, active)
VALUES ('admin:token', 'Admin User', true);

-- Create an identifier (hash of the identifier string)
INSERT INTO identifiers (member_id, phone_hash, is_admin)
SELECT
    id,
    encode(sha256('+1234567890'::bytea), 'hex'),
    true
FROM members WHERE expo_push_token = 'admin:token';
```

### 3. Test Login

With `TEST_IDENTIFIER_ENABLED=true`:
- Phone: `+1234567890`
- Code: any value (Twilio verification is skipped)

## How It Works

1. **Environment Variable** - `ADMIN_IDENTIFIERS` lists whitelisted emails/phones
2. **Database** - Identifiers table has `is_admin` boolean flag
3. **OTP Verification** - Twilio sends verification codes (or skipped for test identifiers)
4. **JWT Token** - Contains `is_admin` claim (24-hour expiry)
5. **Authorization** - Server validates JWT on every request

## Authentication Flow

1. **Login** - Admin visits the admin app → login screen
2. **Send Code** - Enter phone/email → OTP sent via Twilio
3. **Verify** - Enter code → JWT token returned
4. **JWT Storage** - Token stored in client
5. **Requests** - All requests include `Authorization: Bearer <token>`
6. **Validation** - Server validates JWT and checks `is_admin` flag

## Admin-Only Features

The following require admin authentication:
- Approve/reject/edit posts
- Create custom posts
- Manage organizations
- Tag content
- View and manage members

## Security Notes

- Identifier matching is case-insensitive for emails, exact for phones
- JWT tokens expire after 24 hours
- Twilio verifies ownership via OTP
- Admin status stored in database (can't be forged)
- All admin mutations check `ctx.require_admin()`
- Always use HTTPS in production
- Keep `JWT_SECRET` secret and rotate regularly

## Troubleshooting

### "Identifier not registered"
Create an identifier in the database (see SQL above), or restore from `data/local_test_db.sql`.

### "Unauthorized: Admin access required"
1. Check `ADMIN_IDENTIFIERS` includes your email/phone
2. Verify database identifier has `is_admin = true`
3. Log out and log back in (refresh JWT token)

### "OTP failed"
1. Check Twilio credentials are correct
2. Verify identifier format (E.164 for phones, valid email)
3. With `TEST_IDENTIFIER_ENABLED=true`, use `+1234567890` (any code works)

## Related Docs

- [Admin Identifiers Migration](ADMIN_IDENTIFIERS_MIGRATION.md) - Migration from ADMIN_EMAILS to ADMIN_IDENTIFIERS
- [Authentication Security](../security/AUTHENTICATION_SECURITY.md) - Security details
