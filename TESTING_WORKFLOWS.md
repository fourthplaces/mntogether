# Testing Restate Workflows

This guide walks through testing the Restate workflow implementation end-to-end.

## Architecture Overview

```
┌─────────────────────┐
│  GraphQL API        │  :8080
│  (cargo run --bin   │
│   server)           │
└──────────┬──────────┘
           │ HTTP POST
           ↓
┌─────────────────────┐
│  Restate Runtime    │  :9070 (ingress)
│  (docker)           │  :9071 (admin)
└──────────┬──────────┘
           │ HTTP POST
           ↓
┌─────────────────────┐
│  Workflow Server    │  :9080
│  (cargo run --bin   │
│   workflow_server)  │
└─────────────────────┘
```

## Prerequisites

1. **Environment variables** - Copy `.env.example` to `.env` and fill in:
   ```bash
   OPENAI_API_KEY=sk-...
   TAVILY_API_KEY=tvly-...
   TWILIO_ACCOUNT_SID=AC...
   TWILIO_AUTH_TOKEN=...
   TWILIO_VERIFY_SERVICE_SID=VA...
   JWT_SECRET=your-secret-key
   ```

2. **Database** - PostgreSQL with pgvector extension running on port 5432

## Option 1: Docker Compose (Recommended)

### Start All Services

```bash
# Start infrastructure + workflow server
docker-compose up -d postgres redis nats restate workflow-server

# Wait for services to be healthy
docker-compose ps

# Register workflows with Restate
./scripts/register-workflows.sh

# Start API server (in separate terminal)
cd packages/server
cargo run --bin server
```

### Verify Services

```bash
# Check Restate is running
curl http://localhost:9071/health

# Check workflow server is running
curl http://localhost:9080/health || echo "Workflow server running (no /health endpoint)"

# Check API server is running
curl http://localhost:8080/health
```

## Option 2: Local Development (3 Terminals)

### Terminal 1: Infrastructure

```bash
# Start Restate, PostgreSQL, Redis, NATS
docker-compose up postgres redis nats restate
```

### Terminal 2: Workflow Server

```bash
cd packages/server
cargo run --bin workflow_server

# Should see:
# Workflow server listening on 0.0.0.0:9080
```

### Terminal 3: API Server

```bash
cd packages/server

# Run migrations first
cargo run --bin migrate_cli

# Start API
cargo run --bin server

# Should see:
# Server listening on 0.0.0.0:8080
```

### Register Workflows

Once all services are running:

```bash
# From project root
./scripts/register-workflows.sh
```

## Testing Workflows

### 1. Test Auth Workflows (SendOtp + VerifyOtp)

#### Via GraphQL

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

#### Via Direct HTTP (bypassing GraphQL)

```bash
# Send OTP directly to Restate
curl -X POST http://localhost:9070/SendOtp/run \
  -H "Content-Type: application/json" \
  -d '{"phone_number": "+1234567890"}'

# Should return:
# {"phone_number": "+1234567890", "success": true}
```

### 2. Test Crawl Workflow

#### Create a test website first

```graphql
mutation {
  submitWebsite(
    name: "Example Site"
    url: "https://example.com"
    description: "Test site"
  ) {
    id
    domain
  }
}
```

#### Trigger crawl workflow

```graphql
mutation {
  crawlWebsite(websiteId: "<website-id-from-above>") {
    jobId
    sourceId
    status
    message
  }
}
```

#### Via Direct HTTP

```bash
# Start crawl workflow (async - doesn't wait)
curl -X POST http://localhost:9070/CrawlWebsite/run/send \
  -H "Content-Type: application/json" \
  -d '{
    "website_id": "uuid-here",
    "visitor_id": "uuid-here",
    "use_firecrawl": true
  }'
```

## Monitoring & Debugging

### Check Restate Invocations

```bash
# List all invocations
curl http://localhost:9071/invocations

# Get specific invocation status
curl http://localhost:9071/invocations/<invocation-id>
```

### Check Workflow Server Logs

```bash
# Docker
docker logs -f mndigitalaid_workflow_server

# Local
# See Terminal 2 output
```

### Check API Server Logs

```bash
# Docker
docker logs -f mndigitalaid_api

# Local
# See Terminal 3 output
```

### Check Restate Logs

```bash
docker logs -f mndigitalaid_restate
```

## Common Issues

### "Deployment not found" Error

**Symptom:** Restate returns 404 when invoking workflow

**Solution:** Register the workflow server
```bash
./scripts/register-workflows.sh
```

### "Connection refused" to Restate

**Symptom:** API server can't connect to Restate

**Solution:**
- Check Restate is running: `docker-compose ps restate`
- Check RESTATE_URL env var: should be `http://localhost:9070` (local) or `http://restate:9070` (docker)

### Workflow Hangs or Times Out

**Symptom:** Workflow invocation never returns

**Solution:**
- Check workflow server is running and registered
- Check workflow server logs for errors
- Verify database connection (workflows need DB for activities)

### "Authentication required" for Crawl

**Symptom:** crawlWebsite mutation fails with auth error

**Solution:**
- Log in first via `verifyCode` mutation to get auth token
- Pass token in Authorization header: `Authorization: Bearer <token>`

## Next Steps

Once workflows are tested:

1. **Add more workflows** - Follow the pattern in `domains/*/workflows/`
2. **Add workflow tests** - Create integration tests
3. **Monitor in production** - Set up Restate observability
4. **Scale** - Restate handles workflow distribution automatically

## Useful Commands

```bash
# Restart all services
docker-compose restart

# View all logs
docker-compose logs -f

# Rebuild after code changes
docker-compose build api workflow-server

# Clean everything
docker-compose down -v
```
