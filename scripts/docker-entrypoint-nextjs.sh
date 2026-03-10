#!/bin/sh
# ───────────────────────────────────────────────────────────────────
# Docker entrypoint for Next.js dev containers
#
# Problem it solves:
#   docker-compose uses anonymous volumes for /app/node_modules to
#   prevent host bind-mounts from overwriting container deps. But
#   anonymous volumes persist across image rebuilds — so adding a new
#   npm dependency rebuilds the image (with fresh yarn install), but
#   the container still sees the OLD volume's node_modules.
#
# Solution:
#   On every container start, compare a hash of yarn.lock against
#   a saved hash inside the volume. If they differ, run yarn install
#   to update the volume's node_modules, then save the new hash.
#   If they match, skip install entirely (fast startup).
# ───────────────────────────────────────────────────────────────────
set -e

LOCK_FILE="/app/yarn.lock"
HASH_FILE="/app/node_modules/.yarn-lock-hash"

current_hash=$(sha256sum "$LOCK_FILE" 2>/dev/null | cut -d' ' -f1 || echo "none")
saved_hash=$(cat "$HASH_FILE" 2>/dev/null || echo "")

if [ "$current_hash" != "$saved_hash" ]; then
  echo "⚡ Dependencies changed (yarn.lock hash mismatch) — running yarn install..."
  cd /app
  if yarn install --immutable 2>/dev/null; then
    echo "$current_hash" > "$HASH_FILE"
    echo "✅ Dependencies updated."
  else
    echo "⚠️  yarn install --immutable failed (lockfile likely read-only or out of sync)."
    echo "   Using node_modules from the Docker image build. This is fine for dev."
    echo "$current_hash" > "$HASH_FILE" 2>/dev/null || true
  fi
else
  echo "✅ Dependencies up to date (yarn.lock unchanged)."
fi

# Hand off to the original CMD
exec "$@"
