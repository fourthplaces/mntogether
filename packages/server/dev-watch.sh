#!/bin/bash
# Development watch script - runs cargo watch with hot-reload
# Restarts when source files change

set -e

echo "[dev-watch] Starting development server with hot-reload..."
echo "[dev-watch] Watching: /app/packages"
echo "[dev-watch] RUST_LOG=${RUST_LOG:-info}"
echo ""

# Check for required API keys
echo "[dev-watch] Checking environment variables..."
MISSING_REQUIRED=0
MISSING_OPTIONAL=0

# Required API keys
if [ -z "$OPENAI_API_KEY" ]; then
    echo "⚠️  WARNING: OPENAI_API_KEY is not set (required)"
    MISSING_REQUIRED=1
fi

if [ -z "$FIRECRAWL_API_KEY" ]; then
    echo "⚠️  WARNING: FIRECRAWL_API_KEY is not set (required)"
    MISSING_REQUIRED=1
fi

if [ -z "$TWILIO_ACCOUNT_SID" ]; then
    echo "⚠️  WARNING: TWILIO_ACCOUNT_SID is not set (required)"
    MISSING_REQUIRED=1
fi

if [ -z "$TWILIO_AUTH_TOKEN" ]; then
    echo "⚠️  WARNING: TWILIO_AUTH_TOKEN is not set (required)"
    MISSING_REQUIRED=1
fi

if [ -z "$TWILIO_VERIFY_SERVICE_SID" ]; then
    echo "⚠️  WARNING: TWILIO_VERIFY_SERVICE_SID is not set (required)"
    MISSING_REQUIRED=1
fi

if [ -z "$JWT_SECRET" ]; then
    echo "⚠️  WARNING: JWT_SECRET is not set (required)"
    MISSING_REQUIRED=1
fi

# Optional API keys
if [ -z "$TAVILY_API_KEY" ]; then
    echo "ℹ️  INFO: TAVILY_API_KEY is not set (optional)"
    MISSING_OPTIONAL=1
fi

if [ -z "$EXPO_ACCESS_TOKEN" ]; then
    echo "ℹ️  INFO: EXPO_ACCESS_TOKEN is not set (optional)"
    MISSING_OPTIONAL=1
fi

if [ $MISSING_REQUIRED -eq 1 ]; then
    echo ""
    echo "❌ ERROR: Required API keys are missing!"
    echo "The server will fail to start without these keys."
    echo "Please set them in docker-compose.yml or a .env file."
    echo ""
fi

if [ $MISSING_OPTIONAL -eq 1 ]; then
    echo ""
    echo "💡 Optional API keys are not set. Some features may be limited."
    echo ""
fi

# Wait for database to be ready
echo "[dev-watch] Waiting for database..."
until pg_isready -h postgres -p 5432 -U postgres; do
    echo "[dev-watch] Database not ready, waiting..."
    sleep 2
done
echo "[dev-watch] Database is ready!"
echo ""

# Detect package manager (prefer yarn)
if command -v yarn &> /dev/null; then
    PKG_MANAGER="yarn"
    BUILD_CMD="yarn build"
    INSTALL_CMD="yarn install"
else
    PKG_MANAGER="npm"
    BUILD_CMD="npm run build"
    INSTALL_CMD="npm install"
fi
echo "[dev-watch] Using $PKG_MANAGER"
echo ""

# Build frontends on startup (skip if SKIP_FRONTEND_BUILD is set)
if [ "$SKIP_FRONTEND_BUILD" = "true" ]; then
    echo "[dev-watch] Skipping frontend builds (SKIP_FRONTEND_BUILD=true)"
    echo ""
else
    # Build admin-spa on startup
    echo "[dev-watch] Building admin-spa..."
    cd /app/packages/admin-spa
    if [ -f "package.json" ]; then
        # Remove node_modules and lock files to ensure clean ARM64 install
        # (npm has a bug with optional dependencies - https://github.com/npm/cli/issues/4828)
        if [ -d "node_modules" ]; then
            echo "[dev-watch] Removing node_modules and lock files for clean ARM64 install..."
            rm -rf node_modules package-lock.json yarn.lock
        fi

        echo "[dev-watch] Installing admin-spa dependencies..."
        $INSTALL_CMD || {
            echo "[dev-watch] WARNING: $PKG_MANAGER install failed, but continuing..."
        }

        # Build admin-spa
        $BUILD_CMD || {
            echo "[dev-watch] WARNING: Admin-spa build failed, but continuing..."
        }
        echo "[dev-watch] Admin-spa built successfully!"
    else
        echo "[dev-watch] WARNING: admin-spa package.json not found, skipping build"
    fi
    echo ""

    # Build web-app on startup
    echo "[dev-watch] Building web-app..."
    cd /app/packages/web-app
    if [ -f "package.json" ]; then
        # Remove node_modules and lock files to ensure clean ARM64 install
        # (npm has a bug with optional dependencies - https://github.com/npm/cli/issues/4828)
        if [ -d "node_modules" ]; then
            echo "[dev-watch] Removing node_modules and lock files for clean ARM64 install..."
            rm -rf node_modules package-lock.json yarn.lock
        fi

        echo "[dev-watch] Installing web-app dependencies..."
        $INSTALL_CMD || {
            echo "[dev-watch] WARNING: $PKG_MANAGER install failed, but continuing..."
        }

        # Build web-app
        $BUILD_CMD || {
            echo "[dev-watch] WARNING: Web-app build failed, but continuing..."
        }
        echo "[dev-watch] Web-app built successfully!"
    else
        echo "[dev-watch] WARNING: web-app package.json not found, skipping build"
    fi
    echo ""
fi

# Run migrations on startup
echo "[dev-watch] Running database migrations..."
cd /app/packages/server
sqlx migrate run || {
    echo "[dev-watch] WARNING: Migrations failed, but continuing..."
}
echo ""

# Run cargo watch
# Watch only Rust packages to prevent restart loops from frontend builds
echo "[dev-watch] Starting cargo watch..."
exec cargo watch \
    -w /app/packages/server \
    -w /app/packages/twilio-rs \
    -w /app/Cargo.toml \
    -w /app/Cargo.lock \
    -s 'cargo run --manifest-path /app/packages/server/Cargo.toml --bin server'
