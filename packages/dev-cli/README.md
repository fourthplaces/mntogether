# Development CLI

A comprehensive development environment management tool for the MNTogether project.

## Quick Start

```bash
# Run the CLI (builds automatically if needed)
./dev.sh

# Or with a specific command
./dev.sh start          # Start development environment
./dev.sh docker up      # Start Docker containers
./dev.sh test          # Run tests
./dev.sh --help        # Show all available commands
```

## Features

### Environment Management
- **Start/Stop**: Quick commands to start/stop the full development environment
- **Docker**: Manage containers (up, down, restart, rebuild, nuke)
- **Logs**: Follow container logs with auto-reconnect
- **Shell**: Open shell in running containers

### Database
- **Migrate**: Run SQL schema migrations (local or remote)
- **Reset**: Drop and recreate database
- **Seed**: Load test data
- **Psql**: Open PostgreSQL shell

### Code Quality
- **Fmt**: Run code formatters (cargo fmt, prettier)
- **Lint**: Run linters (clippy, eslint)
- **Check**: Run pre-commit checks (fmt + lint + type check)
- **Test**: Run test suites with filtering and watch mode
- **Coverage**: Generate code coverage reports

### Package Commands
Packages define their own commands in `dev.toml` files:

```toml
# packages/server/dev.toml
[cmd.build]
default = "cargo build -p server"
watch = "cargo watch -x 'build -p server'"

[cmd.test]
default = "cargo test -p server"
```

Run with:
```bash
./dev.sh build              # Build all packages
./dev.sh build --watch      # Build with watch mode
./dev.sh cmd typecheck      # Run typecheck command
./dev.sh cmd lint:fix       # Run lint with fix variant
```

### Watch Mode
- Auto-rebuild on file changes
- Supports API, app, or all targets

### Environment Variables
- Pull/push environment variables to/from Pulumi ESC
- Set individual variables
- Show deployment info

### Utilities
- **Doctor**: Check system prerequisites
- **Status**: Show development environment status
- **Sync**: Sync everything (git pull + env + migrate)
- **Init**: First-time developer setup

## Configuration

### Global Configuration (.dev/config.toml)
```toml
[project]
name = "mntogether"

[workspaces]
packages = ["packages/*"]

[environments]
available = ["dev", "prod"]
default = "dev"

[services]
server = 8080
postgres = 5432
redis = 6379
```

### Package Configuration (packages/*/dev.toml)
```toml
# Mark as releasable package
releasable = true

# ECS service name (for CloudWatch logs)
ecs_service = "mntogether-server"

# Commands with variants
[cmd.build]
default = "cargo build -p server"
release = "cargo build -p server --release"
watch = "cargo watch -x 'build -p server'"
```

## Interactive Mode

Run `./dev.sh` without arguments for an interactive menu with:
- Fuzzy search through commands
- Recent command history
- Smart favorites based on usage
- Visual indicators for service status

## Advanced Features

### AI Assistant
- AI-powered code fixes
- Custom lint rules
- Task automation from markdown files

### CI/CD Integration
- View workflow runs
- Watch build status
- Trigger workflows
- Re-run failed jobs

### Release Management
- Interactive package releases
- Semantic versioning (patch, minor, major)
- Git tagging and changelog generation
- Rollback support

### DevOps Mode
```bash
./dev.sh --devops
```
Access to:
- ECS exec (SSH into containers)
- Job queue debugging
- CloudWatch logs
- Remote database shell

## Tips

- Use `./dev.sh --help` to see all available commands
- Most commands have short aliases (e.g., `d` for `docker`, `t` for `test`)
- The CLI remembers your recent actions and suggests them
- Configuration is hierarchical: `.dev/config.toml` → `packages/*/dev.toml`
- All paths and patterns are configurable - no magic strings!

## Development

The CLI itself is a Rust package in `packages/dev-cli`. To work on it:

```bash
# Edit source files in packages/dev-cli/src/
# The wrapper script (dev.sh) auto-rebuilds when needed

# Manual build
cargo build --release --bin dev

# Run directly
cargo run --bin dev -- <args>
```

See `packages/dev-cli/CLAUDE.md` for development guidelines.
