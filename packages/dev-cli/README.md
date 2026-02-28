# dev-cli

Ratatui TUI dashboard for managing Root Editorial's Docker services. Replaces the old bash `dev.sh` interactive mode with double-buffered rendering, async docker polling, and instant key response.

## Build

```bash
cargo build --release -p dev-cli
```

The binary lands at `target/release/dev`. The repo-root `./dev.sh` script delegates to it automatically.

## Usage

```
dev              # Interactive TUI dashboard (default)
dev start        # Start all services (docker compose up -d)
dev stop         # Stop all services (docker compose down)
dev restart      # Restart all services (down + up)
dev status       # One-shot status table (no TUI)
dev logs [svc]   # Follow logs (all or specific service)
```

## Dashboard keys

### Main menu

| Key | Action |
|-----|--------|
| `i` | Manage Infrastructure (Postgres, Restate, MinIO) |
| `b` | Manage Backend (Rust Server) |
| `f` | Manage Frontend (Admin App, Web App) |
| `a` | Manage All Services |
| `1`–`4` | Open service URL in browser |
| `d` | Reset database (drop → create → migrate → seed) |
| `l` | Follow all logs (Ctrl+C to return) |
| `q` | Quit |

### Layer submenu

After pressing `i`/`b`/`f`/`a`, the submenu shows:

| Key | Action |
|-----|--------|
| `s` | Start layer services |
| `x` | Stop layer services |
| `r` | Restart layer services |
| `b` | Rebuild with `--build` (not available for Infra) |
| `l` | Follow logs for this layer |
| `Esc` | Back to main menu |

## Architecture

```
src/
├── main.rs       # Clap CLI, terminal lifecycle, event loop
├── app.rs        # State machine, key handling, operation spawning
├── docker.rs     # Async docker commands (tokio::process::Command)
├── events.rs     # Event pump: keys + 200ms tick + 5s docker refresh → mpsc
├── services.rs   # Static config: services, layers, ports, container names
└── ui.rs         # Ratatui rendering: layer blocks, service table, command bar
```

Three async tasks feed a single `mpsc` channel consumed by the main loop:

1. **Input** — crossterm `EventStream` for keyboard events
2. **Tick** — 200ms interval for animations and message expiry
3. **Docker refresh** — 5s interval polling `docker stats` + `docker inspect` concurrently

All docker operations (start/stop/rebuild) run as background tokio tasks. The UI never blocks — it renders from cached state and updates when refresh results arrive.
