# Development Setup Complete

## 🎉 Summary

Your development CLI is now fully set up with hot-reload support, database migrations, and an interactive menu system.

## ✨ New Features

### 1. **Cargo Watch Hot-Reload** 🔥
The API server now automatically reloads when you edit Rust files:
- Edit any `.rs` file in `packages/`
- Save the file
- Cargo watch rebuilds and restarts the server
- No manual restarts needed!

**New Files:**
- `packages/server/Dockerfile.dev` - Development Dockerfile with cargo-watch
- `packages/server/dev-watch.sh` - Watch script for hot-reload
- Updated `packages/server/docker-compose.yml` - Volume mounts for live code sync

### 2. **Interactive Service Selection** 🎯
When restarting or rebuilding Docker services, you can now select which ones:
- Use **Space** to select/deselect services
- Use **Enter** to confirm
- Available services: postgres, redis, api

### 3. **Database Migrations** 🗄️
New menu option to run migrations on demand:
- Migrations also run automatically when Docker starts
- Uses `sqlx migrate run` inside the API container
- Handles migration failures gracefully

### 4. **GraphQL Playground Launcher** 🌐
One-click access to the GraphQL playground:
- Opens browser to `http://localhost:8080/graphql`
- Helpful error messages if server isn't running
- Manual URL fallback if browser fails

### 5. **Improved Log Viewing** 📋
Better log streaming experience:
- Shows last 100 lines on startup
- Stays attached until you press Ctrl+C
- Clear instructions for stopping
- Returns to menu after stopping

## 📋 Updated Menu

The interactive menu now offers 8 options:

1. 📱 Start mobile (Expo)
2. 🐳 Docker start
3. 🔄 Docker restart (with service selection)
4. 🔨 Docker rebuild (with service selection)
5. 📋 Follow docker logs (attached mode)
6. 🗄️ Run database migrations
7. 🌐 Open GraphQL Playground
8. 🛑 Exit

## 🚀 Quick Start

```bash
# Clone and run
git clone <repo>
cd mndigitalaid
./dev.sh
```

That's it! The CLI handles:
- ✅ Dependency checks
- ✅ First-time setup
- ✅ Interactive menu
- ✅ Hot-reload development
- ✅ Automatic migrations

## 🔥 Hot-Reload Workflow

1. Start Docker: `./dev.sh` → "🐳 Docker start"
2. Edit Rust files in your editor
3. Save the file
4. Watch logs: `./dev.sh` → "📋 Follow docker logs"
5. See automatic rebuild and restart!

No need to manually restart the server after code changes.

## 📁 New Files Created

```
mndigitalaid/
├── dev.sh                              # Main entry point
├── DEV_CLI.md                          # Documentation
├── packages/
│   ├── dev-cli/                        # Rust CLI binary
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── server/
│       ├── Dockerfile.dev              # Dev Dockerfile with cargo-watch
│       ├── dev-watch.sh                # Hot-reload script
│       └── docker-compose.yml          # Updated with volume mounts
└── Cargo.toml                          # Updated workspace
```

## 🎯 Development Best Practices

### When to Rebuild
Only rebuild Docker when:
- Adding new Rust dependencies to `Cargo.toml`
- Modifying `Dockerfile.dev`
- Changing system dependencies

For code changes, cargo-watch handles it automatically!

### Build Performance
- **First build**: ~5-10 minutes (downloads dependencies)
- **Subsequent builds**: ~30-60 seconds (cached)
- **Hot-reload**: ~10-20 seconds (incremental compilation)

Build artifacts are cached in the `mndigitalaid_api_target` Docker volume.

### Service Selection Tips
- **Restart API only**: Usually sufficient for most changes
- **Restart Postgres**: If you modified database config
- **Restart Redis**: If you modified cache settings
- **Rebuild API**: After adding dependencies
- **Rebuild Postgres/Redis**: Rarely needed

## 🔧 Technical Details

### Volume Mounts
The docker-compose setup mounts:
- `Cargo.lock` - Keep lockfile in sync
- `Cargo.toml` - Workspace manifest
- `packages/` - All source code (hot-reload)
- `api-target/` - Cached build artifacts (persisted)

### Cargo Watch Configuration
Watches:
- `/app/packages` - All workspace members
- Runs: `cargo run --bin api`
- Automatically runs migrations on startup
- Waits for database to be ready

### Dependencies
The dev CLI uses:
- `cargo-watch` - File watching and auto-rebuild
- `sqlx-cli` - Database migrations
- `dialoguer` - Interactive prompts
- `colored` - Terminal colors
- `open` - Browser launcher

## 📚 Documentation

- [DEV_CLI.md](DEV_CLI.md) - Complete CLI documentation
- [README.md](README.md) - Project overview

## 🎉 What's Next?

You're all set! Here's what you can do:

1. **Start developing**: Edit Rust files and see changes instantly
2. **Test the API**: Use GraphQL Playground to explore
3. **Build mobile app**: Start Expo and test on device
4. **Monitor logs**: Watch for errors and debug issues
5. **Run migrations**: Add new database schemas easily

Happy coding! 🚀
