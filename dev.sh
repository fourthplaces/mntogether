#!/usr/bin/env bash

# Minnesota Digital Aid Development CLI
# Single entry point for development workflow

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEV_CLI_BIN="$PROJECT_ROOT/target/release/dev"

echo -e "${BLUE}╔══════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Minnesota Digital Aid Dev CLI       ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════╝${NC}"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo is not installed${NC}"
    echo ""
    echo "Please install Rust from: https://rustup.rs/"
    echo ""
    echo "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if dev CLI binary exists and is up to date
NEEDS_BUILD=false

if [ ! -f "$DEV_CLI_BIN" ]; then
    NEEDS_BUILD=true
else
    # Check if source is newer than binary
    if [ "packages/dev-cli/src/main.rs" -nt "$DEV_CLI_BIN" ] || \
       [ "packages/dev-cli/Cargo.toml" -nt "$DEV_CLI_BIN" ]; then
        NEEDS_BUILD=true
    fi
fi

# Build the dev CLI if needed
if [ "$NEEDS_BUILD" = true ]; then
    echo -e "${YELLOW}🔨 Building dev CLI...${NC}"
    echo ""
    cd "$PROJECT_ROOT"
    cargo build --release --bin dev
    echo ""
    echo -e "${GREEN}✅ Build complete${NC}"
    echo ""
fi

# Run the dev CLI
exec "$DEV_CLI_BIN"
