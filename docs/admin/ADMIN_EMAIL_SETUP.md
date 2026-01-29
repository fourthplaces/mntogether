# Admin Email Whitelisting Guide

## Overview

The admin authentication system uses an email whitelist to grant admin privileges. The system is already set up with the infrastructure, but requires manual identifier creation currently.

## Configuration

### Environment Variable

Set the `ADMIN_EMAILS` environment variable with a comma-separated list of admin email addresses:

```bash
# .env or environment
ADMIN_EMAILS=admin@example.com,admin2@example.com,owner@company.org
```

### How It Works

1. **Email Whitelisting** (`src/domains/auth/models/identifier.rs:92-103`)
   - The `is_admin_identifier()` function checks if an email is in the whitelist
   - Case-insensitive matching
   - Only works for email addresses (not phone numbers currently)

2. **Admin Flag Storage** (`migrations/000015_create_identifiers.sql`)
   - Each identifier has an `is_admin` boolean field
   - Stored in the database during identifier creation

3. **JWT Token** (`src/domains/auth/jwt.rs:12`)
   - The `is_admin` flag is included in the JWT claims
   - Server validates this on every request

4. **Authorization Checks** (`src/server/graphql/context.rs:53-61`)
   - `ctx.require_admin()` enforces admin-only mutations
   - Returns `Unauthorized: Admin access required` error if not admin

## Current Setup Required

### Manual Admin Identifier Creation

Currently, identifiers must be created manually in the database. Here's how:

```sql
-- 1. First, ensure you have a member record (or create one)
INSERT INTO members (id, expo_push_token, searchable_text, city, state, active)
VALUES (
    uuid_generate_v4(),
    'AdminPushToken',
    'Admin user',
    'Minneapolis',
    'MN',
    true
);

-- 2. Create an identifier for your admin email
-- Replace 'admin@example.com' with your actual admin email
INSERT INTO identifiers (member_id, phone_hash, is_admin)
VALUES (
    (SELECT id FROM members WHERE expo_push_token = 'AdminPushToken' LIMIT 1),
    encode(sha256('admin@example.com'::bytea), 'hex'),
    true
);
```

### Quick Script

```bash
# Connect to your database
psql $DATABASE_URL

# Create admin identifier (replace EMAIL and CITY/STATE)
\set admin_email 'admin@example.com'
INSERT INTO members (expo_push_token, searchable_text, city, state, active)
VALUES ('AdminToken', 'Admin', 'Minneapolis', 'MN', true);

INSERT INTO identifiers (member_id, phone_hash, is_admin)
SELECT
    id,
    encode(sha256(:'admin_email'::bytea), 'hex'),
    true
FROM members WHERE expo_push_token = 'AdminToken';
```

## Authentication Flow

1. **Login** - Admin visits `/admin` → redirected to login
2. **Send Code** - Enter email → `sendVerificationCode` mutation
3. **Twilio** - Verification code sent via Twilio
4. **Verify** - Enter code → `verifyCode` mutation returns JWT
5. **JWT Storage** - Token stored in `localStorage` as `admin_jwt_token`
6. **Requests** - All GraphQL requests include `Authorization: Bearer <token>`
7. **Validation** - Server validates JWT and checks `is_admin` flag

## Admin-Only Mutations

The following mutations require admin privileges:

- `approveNeed` - Approve a pending need
- `editAndApproveNeed` - Edit and approve a need
- `rejectNeed` - Reject a need
- `scrapeOrganization` - Scrape an organization source
- `createCustomPost` - Create a custom post
- `repostNeed` - Repost an existing need
- `expirePost` - Expire a post
- `archivePost` - Archive a post
- `updateMemberStatus` - Activate/deactivate members
- `createOrganization` - Create new organizations
- `addOrganizationTags` - Tag organizations

## Recommended Improvements

### 1. Automatic Admin Identifier Creation

Add a GraphQL mutation or startup script to automatically create admin identifiers:

```rust
// In src/domains/auth/effects.rs or similar
pub async fn ensure_admin_identifiers(
    admin_emails: &[String],
    pool: &PgPool
) -> Result<()> {
    for email in admin_emails {
        let phone_hash = hash_phone_number(email);

        // Check if identifier already exists
        if !Identifier::exists(&phone_hash, pool).await? {
            // Create admin member if needed
            let member_id = create_admin_member(email, pool).await?;

            // Create admin identifier
            Identifier::create(member_id, phone_hash, true, pool).await?;
            info!("Created admin identifier for {}", email);
        }
    }
    Ok(())
}
```

### 2. Admin Phone Numbers

Currently only emails are whitelisted. To support admin phone numbers:

```bash
# Add to .env
ADMIN_PHONES=+15551234567,+15559876543
```

### 3. Database Seeding

Add a seed script that runs on first startup:

```bash
# packages/server/seeds/001_create_admins.sql
-- Automatically create admin identifiers from ADMIN_EMAILS
```

## Security Notes

1. **JWT Expiration** - Tokens expire after 24 hours
2. **Secret Rotation** - Change `JWT_SECRET` to invalidate all tokens
3. **HTTPS Only** - Always use HTTPS in production
4. **Email Verification** - Twilio verifies email ownership via OTP
5. **Case Insensitive** - Email matching is case-insensitive for security

## Testing

### Development Mode

Enable test identifier for easier testing:

```bash
TEST_IDENTIFIER_ENABLED=true
```

Then use `test@example.com` with code `123456` for testing.

**WARNING**: Never enable in production!
