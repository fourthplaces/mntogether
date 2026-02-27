# Testing Restate Workflows

How to test Restate workflows end-to-end in development.

## Architecture

```
┌─────────────────────┐
│  Next.js Apps       │  :3000 (admin), :3001 (web)
│  (GraphQL clients)  │
└──────────┬──────────┘
           │ HTTP POST
           ↓
┌─────────────────────┐
│  Restate Runtime    │  :8180 (ingress), :9070 (admin)
│  (docker)           │
└──────────┬──────────┘
           │ h2c
           ↓
┌─────────────────────┐
│  Rust Server        │  :9080
│  (services +        │
│   auto-registers)   │
└──────────┬──────────┘
           │
           ↓
┌─────────────────────┐
│  PostgreSQL         │  :5432
└─────────────────────┘
```

## Quick Start

```bash
# Start all services (Postgres, Restate, Rust server)
make up
# or: docker compose up -d

# Server auto-registers with Restate on startup — no manual registration needed.
```

## Testing Auth Workflows (SendOtp + VerifyOtp)

### Via GraphQL

```graphql
# Send OTP
mutation {
  sendVerificationCode(phoneNumber: "+1234567890")
}

# Verify OTP (use code from Twilio SMS)
mutation {
  verifyCode(phoneNumber: "+1234567890", code: "123456")
}
```

### Via Direct HTTP

```bash
# Send OTP directly to Restate ingress
curl -X POST http://localhost:8180/SendOtp/run \
  -H "Content-Type: application/json" \
  -d '{"phone_number": "+1234567890"}'
```

## Monitoring & Debugging

### Check Restate Invocations

```bash
# List all invocations
curl http://localhost:9070/invocations

# Get specific invocation status
curl http://localhost:9070/invocations/<invocation-id>
```

### Logs

```bash
make logs-server    # Rust server logs
make logs-db        # PostgreSQL logs
make logs           # All services

# Or directly:
docker compose logs -f server
docker compose logs -f restate
```

## Common Issues

### "Deployment not found" Error

**Symptom:** Restate returns 404 when invoking workflow

**Solution:** The server auto-registers on startup. Restart it:
```bash
make restart-server
```

### "Connection refused" to Restate

**Symptom:** Server can't connect to Restate

**Solution:**
- Check Restate is running: `docker compose ps restate`
- Check `RESTATE_URL` env var: `http://restate:8080` (docker) or `http://localhost:8180` (local)

### Workflow Hangs or Times Out

**Symptom:** Invocation never returns

**Solution:**
- Check server logs for errors: `make logs-server`
- Verify database connection (workflows need DB for activities)
- Check Restate admin: `curl http://localhost:9070/invocations`
