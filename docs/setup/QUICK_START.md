# Quick Start Guide

Get the full stack running in 2 minutes.

## 1. Start Backend API

```bash
cd packages/server

# First time only
cp .env.example .env
# Edit .env with your API keys

# Start dependencies
docker-compose up -d

# Run server
cargo run --bin server
```

✅ API available at: http://localhost:8080/graphql
✅ GraphQL Playground: http://localhost:8080/graphql (in browser)
✅ Health check: http://localhost:8080/health

## 2. Start Frontend

```bash
cd packages/web-next

# First time only
yarn install

# Start dev server
yarn dev
```

✅ Frontend available at: http://localhost:3000

## 3. Test It Works

Open http://localhost:3000 and try searching for "food assistance".

Or test directly from browser console:

```javascript
fetch('http://localhost:8080/graphql', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    query: '{ organizations { id name } }'
  })
})
.then(r => r.json())
.then(console.log)
```

## Architecture

```
┌─────────────────┐          ┌─────────────────┐
│   Next.js       │  HTTP    │   Rust API      │
│   localhost:3000│ ──────>  │   localhost:8080│
│                 │ GraphQL  │                 │
└─────────────────┘          └─────────────────┘
```

Frontend makes direct HTTP requests to backend. No proxy needed.

## CORS

Already configured! In development, the Rust server automatically allows:
- http://localhost:3000 (Next.js)
- http://localhost:19006 (Expo)
- http://localhost:8081 (React Native)

## Common Issues

### CORS Error

```
Access to fetch has been blocked by CORS policy
```

**Fix:**
1. Check backend is running: `curl http://localhost:8080/health`
2. Restart backend after changing .env

### Can't Connect to API

```
Failed to fetch
```

**Fix:**
1. Check `NEXT_PUBLIC_API_URL` in `packages/web-next/.env.local`
2. Default should be: `http://localhost:8080/graphql`

## Next Steps

- **API Docs:** [docs/API_INTEGRATION_GUIDE.md](docs/API_INTEGRATION_GUIDE.md)
- **Deployment:** [DEPLOYMENT.md](DEPLOYMENT.md)
- **Frontend README:** [packages/web-next/README.md](packages/web-next/README.md)
