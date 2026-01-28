# Dev CLI

Interactive development CLI for Minnesota Digital Aid project.

## Installation

```bash
# Build the CLI
cargo build --bin dev

# Run the CLI
cargo run --bin dev
```

## Features

### 📱 Start mobile (Expo)
Launches the Expo development server for the mobile app.

### 🐳 Docker Operations
- **Docker start** - Start all Docker services (PostgreSQL, Redis, API)
- **Docker restart** - Restart selected services
- **Docker rebuild** - Rebuild and restart selected services
- **Follow docker logs** - Stream logs from all services

### 🗄️ Database
- **Run database migrations** - Execute pending migrations in the database

### 🔑 Environment Variables

#### Check API keys status
Shows which required and optional environment variables are set/missing.

#### 📝 Setup environment variables (wizard)
**Interactive setup wizard** that walks you through each environment variable:

1. Shows current value (masked for security)
2. Lets you:
   - Keep existing value
   - Update to new value
   - Skip (leave empty for optional vars)
3. Saves all values to `packages/server/.env`
4. Optionally pushes to Fly.io

**Required variables:**
- `OPENAI_API_KEY` - OpenAI API for AI features
- `FIRECRAWL_API_KEY` - Firecrawl for web scraping
- `TWILIO_ACCOUNT_SID` - Twilio for SMS
- `TWILIO_AUTH_TOKEN` - Twilio authentication
- `TWILIO_VERIFY_SERVICE_SID` - Twilio verify service
- `JWT_SECRET` - JWT token signing (random 32+ char string)

**Optional variables:**
- `TAVILY_API_KEY` - Tavily search API
- `EXPO_ACCESS_TOKEN` - Expo push notifications
- `CLERK_SECRET_KEY` - Clerk authentication

### 🚀 Manage Fly.io environment variables
Interactive submenu for managing Fly.io secrets:

- **List current secrets** - View all secrets on Fly.io
- **Set a secret** - Add/update a single secret
- **Pull secrets to .env** - View secret names (values not retrievable)
- **Push secrets from .env** - Upload all secrets from local .env to Fly.io

### 👤 Manage admin users
Interactive submenu for managing admin email whitelist:

- **Show current admin emails** - View configured admin emails from .env
- **Add admin email** - Add new admin (validates email format)
- **Remove admin email** - Remove existing admin from list
- **Save to local .env** - Persist admin emails to `ADMIN_EMAILS` variable
- **Push to Fly.io** - Deploy admin emails to production (updates `ADMIN_EMAILS` secret)

**Admin Authentication:**
- Admins are authenticated via email + OTP code
- `ADMIN_EMAILS` environment variable whitelists authorized emails
- Emails are case-insensitive
- Separate identifiers must be created in the database (see docs)

### 🌐 Open GraphQL Playground
Opens the GraphQL playground in your default browser.

## Quick Start

1. **First run:**
   ```bash
   cargo run --bin dev
   ```

2. **Setup environment variables:**
   - Select "📝 Setup environment variables (wizard)"
   - Follow prompts to set each variable
   - Variables are saved to `packages/server/.env`

3. **Start Docker services:**
   - Select "🐳 Docker start"
   - Services available at:
     - API: http://localhost:8080
     - PostgreSQL: localhost:5432
     - Redis: localhost:6379

4. **View logs:**
   - Select "📋 Follow docker logs"
   - Press Ctrl+C to return to menu

## Environment Setup Example

```bash
# Run the wizard
cargo run --bin dev
# Select: 📝 Setup environment variables (wizard)

# For each variable:
# - Shows help text with where to get the value
# - Shows current value (if set)
# - Lets you update, keep, or skip

# Example wizard flow:
────────────────────────────────────────────────────────────
🔴 OPENAI_API_KEY
   Required: OpenAI API key for AI features
   Help: Get from https://platform.openai.com/api-keys
   Current: Not set

What would you like to do with OPENAI_API_KEY?
> Set value now
  Skip (leave empty)

Enter value for OPENAI_API_KEY: sk-proj-...
   ✓ Value updated
```

## Tips

- Use the wizard for initial setup - it's the easiest way to configure all variables
- Check API key status before starting Docker to see what's missing
- Push to Fly.io after setting local variables for production deployment
- The wizard masks sensitive values for security (shows first/last 4 chars only)
