# API Integration Guide

Guide for integrating the Next.js frontend apps with the Rust backend via GraphQL.

## Architecture Overview

```
┌──────────────────┐     ┌──────────────────┐
│ Admin App / Web  │     │  Restate Runtime  │     ┌──────────────────┐
│  (Next.js)       │────▶│  (Port 8180)      │────▶│  Rust Server     │
│  Port 3000/3001  │     │                   │     │  (Port 9080)     │
└──────────────────┘     └──────────────────┘     └──────────────────┘
```

The frontend apps communicate with the backend through GraphQL. The `shared` package (`packages/shared/`) defines the GraphQL schema types used by both admin-app and web-app.

## CORS Configuration

### Development (Automatic)

In debug builds, CORS automatically allows:
- `http://localhost:3000` (Admin App)
- `http://localhost:3001` (Web App)

**No configuration needed** — just start both servers!

### Production (Manual)

Set `ALLOWED_ORIGINS` in `.env`:

```bash
ALLOWED_ORIGINS=https://yourdomain.org,https://www.yourdomain.org
```

**Security:** Never use `*` in production. Always whitelist specific domains.

## Quick Start

### 1. Start Backend

```bash
# Start infrastructure
docker compose up -d

# Run server
cargo run --bin server
```

Server available at: `http://localhost:9080`

### 2. Start Frontend

```bash
# Admin app
cd packages/admin-app && yarn dev    # Port 3000

# Public web app
cd packages/web-app && yarn dev      # Port 3001
```

### 3. Test Connection

```bash
# Test health endpoint
curl http://localhost:9080/health
```

## GraphQL Client Usage

### TypeScript Client

Both apps use the shared GraphQL types from `packages/shared/`.

#### Server-Side Queries (Server Components)

```typescript
import { graphqlFetch } from "@/lib/graphql";

export default async function Page() {
  const data = await graphqlFetch(SOME_QUERY, {
    variables: { limit: 10 },
    revalidate: 60,  // Cache for 60 seconds
  });

  return <div>{/* render data */}</div>;
}
```

#### Authenticated Requests

```typescript
// Get token from auth (after OTP login)
const token = localStorage.getItem("auth_token");

const data = await graphqlFetchClient(
  ADMIN_MUTATION,
  { input: { ... } },
  token  // Pass token for authentication
);
```

## Authentication

```typescript
// Send verification code
SEND_VERIFICATION_CODE
- Variables: { phoneNumber: string }
- Returns: boolean

// Verify code and get JWT token
VERIFY_CODE
- Variables: { phoneNumber: string, code: string }
- Returns: string (JWT token)
```

## Environment Variables

### Frontend (Next.js)

```bash
# packages/admin-app/.env.local or packages/web-app/.env.local
NEXT_PUBLIC_API_URL=http://localhost:9080/graphql
```

### Backend (Rust)

```bash
# .env
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:3001
```

## Troubleshooting

### CORS Error

**Error:** `Access to fetch has been blocked by CORS policy`

**Solutions:**
1. Check Rust server is running: `curl http://localhost:9080/health`
2. Verify `ALLOWED_ORIGINS` includes your frontend URL
3. Restart Rust server after changing `.env`

### 401 Unauthorized

**Solution:**
- Admin mutations require JWT token
- Get token via authentication flow (send code → verify → store JWT)

### Network Error

**Solutions:**
1. Check backend is running: `curl http://localhost:9080/health`
2. Check frontend API URL config
3. Verify Restate runtime is running: `curl http://localhost:9070`

## Best Practices

1. **Error Handling** — Always handle errors in try-catch
2. **Loading States** — Show loading indicators during fetches
3. **Caching** — Use Next.js `revalidate` for server components
4. **Type Safety** — Use the shared GraphQL types from `packages/shared/`
5. **Auth Tokens** — Store in localStorage, include in requests, clear on logout

## Related Documentation

- [Authentication Security](../security/AUTHENTICATION_SECURITY.md) — Auth details
- [Package Structure](../architecture/PACKAGE_STRUCTURE.md) — Monorepo layout
