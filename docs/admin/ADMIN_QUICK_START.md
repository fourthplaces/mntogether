# Admin Authentication Quick Start

## Overview

Admin emails are whitelisted via the `ADMIN_EMAILS` environment variable. Use the dev CLI to manage admins easily.

## Quick Setup

### 1. Run the Dev CLI

```bash
./dev.sh
```

### 2. Select "👤 Manage admin users"

The CLI provides an interactive menu to:

- **📋 Show current admin emails** - View configured admins
- **➕ Add admin email** - Add new admin (validates email format)
- **➖ Remove admin email** - Remove existing admin
- **💾 Save to local .env** - Persist changes locally
- **⬆️ Push to Fly.io** - Deploy to production

### 3. Add Your Admin Email

```
1. Select "➕ Add admin email"
2. Enter your email: admin@example.com
3. Select "💾 Save to local .env"
4. Select "⬆️ Push to Fly.io" (for production)
```

## Manual Setup

### Local Development (.env)

Add to `packages/server/.env`:

```bash
# Admin emails (comma-separated, case-insensitive)
ADMIN_EMAILS=admin@example.com,admin2@example.com

# Development testing (NEVER in production!)
TEST_IDENTIFIER_ENABLED=true
```

### Production (Fly.io)

```bash
flyctl secrets set ADMIN_EMAILS=admin@example.com,admin2@example.com
```

## Creating Admin Identifiers in Database

Admins must have an identifier in the database. Currently manual:

```sql
-- Connect to your database
psql $DATABASE_URL

-- Create a member for the admin
INSERT INTO members (expo_push_token, searchable_text, city, state, active)
VALUES ('AdminToken', 'Admin User', 'Minneapolis', 'MN', true);

-- Create an identifier for the admin email
-- Replace 'admin@example.com' with your actual admin email
INSERT INTO identifiers (member_id, phone_hash, is_admin)
SELECT
    id,
    encode(sha256('admin@example.com'::bytea), 'hex'),
    true
FROM members WHERE expo_push_token = 'AdminToken';
```

### Quick SQL Script

```bash
# Set your admin email
ADMIN_EMAIL="admin@example.com"

# Run the SQL
psql $DATABASE_URL <<EOF
INSERT INTO members (expo_push_token, searchable_text, city, state, active)
VALUES ('AdminToken', 'Admin', 'Minneapolis', 'MN', true);

INSERT INTO identifiers (member_id, phone_hash, is_admin)
SELECT
    id,
    encode(sha256('${ADMIN_EMAIL}'::bytea), 'hex'),
    true
FROM members WHERE expo_push_token = 'AdminToken';
EOF
```

## Testing

### Development Mode

With `TEST_IDENTIFIER_ENABLED=true`, use:

- Email: `test@example.com`
- Code: `123456`

**⚠️ NEVER enable in production!**

### Production Mode

1. Visit `https://yourdomain.com/admin`
2. Enter your admin email
3. Enter the OTP code sent by Twilio
4. You're authenticated!

## How It Works

1. **Environment Variable** - `ADMIN_EMAILS` lists whitelisted emails
2. **Database** - Identifiers table has `is_admin` boolean flag
3. **OTP Verification** - Twilio sends verification codes
4. **JWT Token** - Contains `is_admin` claim (24-hour expiry)
5. **Authorization** - Server validates JWT on every request

## Admin-Only Features

The following require admin authentication:

- Approve/reject/edit needs
- Scrape organizations
- Create/repost/expire posts
- Manage members and organizations
- Tag organizations

## Security Notes

- ✅ Email matching is case-insensitive
- ✅ JWT tokens expire after 24 hours
- ✅ Twilio verifies email ownership via OTP
- ✅ Admin status stored in database (can't be forged)
- ✅ All mutations check `ctx.require_admin()`
- 🔒 Always use HTTPS in production
- 🔒 Keep `JWT_SECRET` secret and rotate regularly

## Troubleshooting

### "Identifier not registered"

Create an identifier in the database (see SQL above).

### "Unauthorized: Admin access required"

1. Check `ADMIN_EMAILS` includes your email
2. Verify database identifier has `is_admin = true`
3. Log out and log back in (refresh JWT token)

### "OTP failed"

1. Check Twilio credentials are correct
2. Verify email address format
3. Check Twilio service is active

## Files Modified

- ✅ Admin SPA built and embedded in server binary
- ✅ JWT authentication with Bearer token
- ✅ Login page with OTP flow
- ✅ Protected routes (redirect to login)
- ✅ Apollo client includes Authorization header
- ✅ Dev CLI admin management commands
- ✅ `.env.example` includes `ADMIN_EMAILS`

## Next Steps

1. Run `./dev.sh` and add your admin email
2. Create database identifier for your email
3. Rebuild admin-spa: `cd packages/admin-spa && yarn build`
4. Rebuild server: `cd packages/server && docker-compose up --build`
5. Visit `http://localhost:8080/admin` and test login

## Support

For detailed information, see:
- `ADMIN_EMAIL_SETUP.md` - Comprehensive setup guide
- `packages/server/.env.example` - All environment variables
