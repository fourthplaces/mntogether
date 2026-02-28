#!/usr/bin/env bash
# Root Editorial - Dev Dashboard
# Delegates to the compiled Ratatui TUI binary (packages/dev-cli).
# Falls back to cargo run if the binary hasn't been built yet.
#
# First-time setup:
#   cargo build --release -p dev-cli
#
# Usage: ./dev.sh              Start services + live dashboard
#        ./dev.sh status       One-shot status check
#        ./dev.sh start        Start services (no dashboard)
#        ./dev.sh stop         Stop all services
#        ./dev.sh restart      Restart all services
#        ./dev.sh logs [svc]   Follow logs (all or specific service)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEV_BIN="$REPO_ROOT/target/release/dev"

# Use the compiled binary if available
if [[ -x "$DEV_BIN" ]]; then
    exec "$DEV_BIN" "$@"
fi

# Fall back to cargo run
if command -v cargo >/dev/null 2>&1; then
    echo "  Building dev-cli (first run)..."
    cargo build --release -p dev-cli 2>&1 && exec "$DEV_BIN" "$@"
fi

echo ""
echo "  dev-cli is not built. Run:"
echo "    cargo build --release -p dev-cli"
echo ""
exit 1
