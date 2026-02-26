# Quick Start Guide

Get the full stack running in 2 minutes.

## Prerequisites

- Docker Desktop installed and running
- `.env` file in project root (copy from `.env.example`, set `JWT_SECRET` and `TEST_IDENTIFIER_ENABLED=true`)

## Start Everything

```bash
./dev.sh
```

This starts all services (PostgreSQL, Redis, Restate, Rust server, Admin app, Web app) and opens a live dashboard showing their status. The first run pulls Docker images and compiles the server -- allow 2-3 minutes.

Dashboard shortcuts:
- `[s]` start all services
- `[r]` restart everything
- `[b]` rebuild the Rust server
- `[l]` follow logs (Ctrl+C to return)
- `[q]` quit

You can also run individual commands:

```bash
./dev.sh start      # Start services without dashboard
./dev.sh stop       # Stop everything
./dev.sh restart    # Restart everything
./dev.sh status     # One-shot status check
./dev.sh logs       # Follow all logs
```

## What's Running

| Service | URL | What it does |
|---------|-----|-------------|
| Admin App (CMS) | http://localhost:3000 | Where you edit and publish content |
| Web App | http://localhost:3001 | Public-facing site |
| Rust Server | :9080 | Backend API (talks to Restate) |
| Restate | :8180 | Durable workflow runtime |
| PostgreSQL | :5432 | Database |
| Redis | :6379 | Cache |

## Test Login

With `TEST_IDENTIFIER_ENABLED=true` in `.env`:
- Phone: `+1234567890`
- Code: any value

## Common Issues

### Everything shows "stopped"

Docker Desktop probably isn't running. Start it and run `./dev.sh` again.

### Server shows "starting" for a long time

First-time Rust compilation takes 2-3 minutes inside Docker. Check progress with `[l]` in the dashboard or `docker compose logs -f server`.

### CORS Error

```
Access to fetch has been blocked by CORS policy
```

The Rust server automatically allows localhost:3000 and localhost:3001 in development. If you see this, the server likely isn't running yet -- check the dashboard.

### Can't Connect to API

Make sure the Rust server shows "OK" in the dashboard. If it shows "FAIL", press `[b]` to rebuild.

## Next Steps

- **API Docs:** [API Integration Guide](../guides/API_INTEGRATION_GUIDE.md)
- **Full Setup:** [Local Dev Setup](LOCAL_DEV_SETUP.md)
- **Deployment:** [Deployment Guide](DEPLOYMENT.md)
