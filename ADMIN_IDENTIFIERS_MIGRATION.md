# Admin Identifiers Migration

Renamed `ADMIN_EMAILS` to `ADMIN_IDENTIFIERS` to support both email addresses and phone numbers for admin authentication.

## What Changed

### Environment Variable
- **Before:** `ADMIN_EMAILS=admin@example.com`
- **After:** `ADMIN_IDENTIFIERS=admin@example.com,+15551234567`

Now supports:
- ✅ **Email addresses** - Case-insensitive matching (admin@example.com)
- ✅ **Phone numbers** - E.164 format (+15551234567)
- ✅ **Mixed list** - Can have both emails and phones in the same variable

### Configuration

**File:** `packages/server/src/config.rs`
```rust
// Before
pub admin_emails: Vec<String>

// After
pub admin_identifiers: Vec<String>
```

Loaded from: `env::var("ADMIN_IDENTIFIERS")`

### Auth Logic

**File:** `packages/server/src/domains/auth/models/identifier.rs`

Updated `is_admin_identifier()` function:
```rust
pub fn is_admin_identifier(identifier: &str, admin_identifiers: &[String]) -> bool {
    admin_identifiers.iter().any(|admin_id| {
        // Case-insensitive match for emails
        if identifier.contains('@') && admin_id.contains('@') {
            admin_id.eq_ignore_ascii_case(identifier)
        } else {
            // Exact match for phone numbers
            admin_id == identifier
        }
    })
}
```

**Matching Rules:**
- **Emails:** Case-insensitive (ADMIN@EXAMPLE.COM matches admin@example.com)
- **Phones:** Exact match (+15551234567 must match exactly)

### Dev CLI

**Menu Text Updated:**
- Before: "Admin emails"
- After: "Admin identifiers"

**Help Text:**
```
Admin users are managed via the ADMIN_IDENTIFIERS environment variable.
Supports emails and phone numbers (E.164: +1234567890)
```

**Commands:**
- `./dev.sh` → "👤 Manage admin users" → Now manages identifiers
- Add/remove identifiers (auto-saves to `.env`)
- Push to Fly.io secrets

## Files Changed

1. **`packages/server/src/config.rs`**
   - Renamed `admin_emails` → `admin_identifiers`
   - Updated env var loading

2. **`packages/server/src/domains/auth/models/identifier.rs`**
   - Updated `is_admin_identifier()` function signature
   - Added support for phone number matching
   - Updated tests to cover emails, phones, and mixed lists

3. **`packages/server/src/domains/auth/effects.rs`**
   - Updated all references to use `admin_identifiers`

4. **`packages/server/src/domains/organization/effects/deps.rs`**
   - Updated `ServerDeps` struct field

5. **`packages/server/src/server/app.rs`**
   - Updated function parameters

6. **`packages/server/src/server/main.rs`**
   - Updated config usage

7. **`packages/server/src/server/auth/edges.rs`**
   - Updated comment

8. **`packages/dev-cli/src/main.rs`**
   - Updated all menu text and prompts
   - Changed env var name in all functions

9. **Environment Files:**
   - `packages/server/.env`
   - `packages/server/.env.example`
   - `packages/server/docker-compose.yml`

## Migration Guide

### For Local Development

1. **Update `.env` file:**
   ```bash
   # Before
   ADMIN_EMAILS=admin@example.com

   # After
   ADMIN_IDENTIFIERS=admin@example.com
   ```

2. **Or add phone number:**
   ```bash
   ADMIN_IDENTIFIERS=+15551234567
   ```

3. **Or use both:**
   ```bash
   ADMIN_IDENTIFIERS=admin@example.com,+15551234567
   ```

4. **Restart server:**
   ```bash
   docker compose restart api
   ```

### For Fly.io Production

1. **Using Dev CLI:**
   ```bash
   ./dev.sh
   # Select: 👤 Manage admin users
   # Select: ⬆️  Push to Fly.io (production)
   ```

2. **Or using flyctl directly:**
   ```bash
   flyctl secrets set ADMIN_IDENTIFIERS=admin@example.com,+15551234567
   ```

3. **Check current value:**
   ```bash
   flyctl secrets list
   ```

### Phone Number Format

Phone numbers **must** be in E.164 format:
- ✅ **Correct:** `+15551234567` (starts with `+`, includes country code)
- ❌ **Wrong:** `5551234567` (missing `+` and country code)
- ❌ **Wrong:** `(555) 123-4567` (not E.164 format)

**Examples:**
- US: `+15551234567`
- UK: `+442012345678`
- International: `+[country code][number]`

## Testing

All tests pass with the new implementation:

```bash
cd packages/server
cargo test domains::auth::models::identifier::tests
```

**Test Coverage:**
- ✅ Email matching (case-insensitive)
- ✅ Phone number matching (exact)
- ✅ Mixed list (emails + phones)
- ✅ Negative cases (non-admins rejected)

## Backwards Compatibility

⚠️ **Breaking Change:** The environment variable name changed from `ADMIN_EMAILS` to `ADMIN_IDENTIFIERS`.

**Required Actions:**
1. Update `.env` file in `packages/server/`
2. Update Fly.io secrets
3. Update any deployment scripts or CI/CD pipelines

## Benefits

1. **Flexibility** - Admins can use phone numbers instead of emails
2. **Better UX** - Phone-based auth often faster than email OTP
3. **Future-proof** - Easy to add other identifier types later
4. **Clearer naming** - "Identifiers" more accurately describes the concept

## Example Configurations

### Email Only (Original)
```bash
ADMIN_IDENTIFIERS=admin@example.com,owner@example.com
```

### Phone Only (New)
```bash
ADMIN_IDENTIFIERS=+15551234567,+15559876543
```

### Mixed (Recommended)
```bash
ADMIN_IDENTIFIERS=admin@example.com,+15551234567,owner@example.com,+15559876543
```

## Rollback

If you need to rollback (not recommended as code expects `ADMIN_IDENTIFIERS`):

1. Revert commits related to this migration
2. Update `.env` back to `ADMIN_EMAILS`
3. Rebuild and redeploy

However, it's better to just update the environment variable as shown above.
