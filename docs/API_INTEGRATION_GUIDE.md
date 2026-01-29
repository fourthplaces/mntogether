# API Integration Guide

Complete guide for integrating the Next.js frontend with the Rust GraphQL API.

## Architecture Overview

```
┌─────────────────┐          ┌─────────────────┐
│   Next.js App   │  HTTP    │   Rust API      │
│   (Port 3000)   │ ──────>  │   (Port 8080)   │
│                 │ GraphQL  │                 │
└─────────────────┘          └─────────────────┘
```

The frontend makes direct HTTP requests to the backend GraphQL API. No proxy or compilation needed.

## CORS Configuration

### How It Works

CORS (Cross-Origin Resource Sharing) is already configured in the Rust server at `packages/server/src/server/app.rs:167-186`.

### Development (Automatic)

In debug builds, CORS automatically allows:
- `http://localhost:3000` (Next.js)
- `http://localhost:19006` (Expo web)
- `http://localhost:8081` (React Native)

**No configuration needed** - just start both servers!

### Production (Manual)

Set `ALLOWED_ORIGINS` in `packages/server/.env`:

```bash
ALLOWED_ORIGINS=https://mndigitalaid.org,https://www.mndigitalaid.org
```

**Security:** Never use `*` in production. Always whitelist specific domains.

## Quick Start

### 1. Start Backend

```bash
cd packages/server

# Copy environment template (first time only)
cp .env.example .env

# Edit .env with your API keys
nano .env

# Start dependencies
docker-compose up -d

# Run server
cargo run --bin server
```

Server available at: `http://localhost:8080/graphql`

### 2. Start Frontend

```bash
cd packages/web-next

# Install dependencies (first time only)
yarn install

# Start dev server
yarn dev
```

Frontend available at: `http://localhost:3000`

### 3. Test Connection

Open browser console at `http://localhost:3000` and run:

```javascript
// Test API connection
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

## GraphQL Client Usage

### TypeScript Client (`lib/graphql.ts`)

The GraphQL client is already set up with type-safe queries and mutations.

#### Server-Side Queries (Server Components)

```typescript
import { graphqlFetch, SEARCH_ORGANIZATIONS } from "@/lib/graphql";
import type { SearchOrganizationsResult } from "@/lib/types";

export default async function Page() {
  const data = await graphqlFetch<SearchOrganizationsResult>(
    SEARCH_ORGANIZATIONS,
    {
      query: "food assistance",
      limit: 10
    },
    {
      revalidate: 60, // Cache for 60 seconds
    }
  );

  return (
    <div>
      {data.searchOrganizationsSemantic.map((match) => (
        <div key={match.organization.id}>
          <h3>{match.organization.name}</h3>
          <p>Similarity: {match.similarityScore.toFixed(2)}</p>
        </div>
      ))}
    </div>
  );
}
```

#### Client-Side Queries (Client Components)

```typescript
"use client";

import { useState } from "react";
import { graphqlFetchClient, SEARCH_ORGANIZATIONS } from "@/lib/graphql";
import type { SearchOrganizationsResult } from "@/lib/types";

export function SearchComponent() {
  const [results, setResults] = useState<SearchOrganizationsResult | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSearch = async (query: string) => {
    setLoading(true);
    try {
      const data = await graphqlFetchClient<SearchOrganizationsResult>(
        SEARCH_ORGANIZATIONS,
        { query, limit: 10 }
      );
      setResults(data);
    } catch (error) {
      console.error("Search failed:", error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div>
      <input
        type="text"
        onChange={(e) => handleSearch(e.target.value)}
        placeholder="Search organizations..."
      />
      {loading && <div>Loading...</div>}
      {results?.searchOrganizationsSemantic.map((match) => (
        <div key={match.organization.id}>
          <h3>{match.organization.name}</h3>
        </div>
      ))}
    </div>
  );
}
```

#### Authenticated Requests

```typescript
import { graphqlFetchClient, CREATE_LISTING } from "@/lib/graphql";

// Get token from auth (after login)
const token = localStorage.getItem("auth_token");

const newListing = await graphqlFetchClient(
  CREATE_LISTING,
  {
    input: {
      organizationId: "org-123",
      listingType: "service",
      title: "Legal Aid Services",
      description: "Free legal consultations",
      category: "legal",
    },
  },
  token // Pass token for authentication
);
```

## Available Queries & Mutations

### Public Queries (No Auth Required)

```typescript
// Search organizations with AI
SEARCH_ORGANIZATIONS
- Variables: { query: string, limit?: number }
- Returns: OrganizationMatch[]

// Get all verified organizations
GET_ORGANIZATIONS
- Variables: none
- Returns: Organization[]

// Get organization by ID
GET_ORGANIZATION
- Variables: { id: string }
- Returns: Organization | null

// Get published posts
GET_PUBLISHED_POSTS
- Variables: { limit?: number }
- Returns: Post[]

// Search listings
SEARCH_LISTINGS
- Variables: { listingType?, category?, capacityStatus?, limit?, offset? }
- Returns: Listing[]
```

### Public Mutations

```typescript
// Submit resource link for scraping
SUBMIT_RESOURCE_LINK
- Variables: { input: SubmitResourceLinkInput }
- Returns: { success: boolean, message: string }

// Track post view (analytics)
TRACK_POST_VIEW
- Variables: { postId: string }
- Returns: boolean

// Track post click (analytics)
TRACK_POST_CLICK
- Variables: { postId: string }
- Returns: boolean
```

### Authentication

```typescript
// Send SMS verification code
SEND_VERIFICATION_CODE
- Variables: { phoneNumber: string }
- Returns: boolean

// Verify code and get JWT token
VERIFY_CODE
- Variables: { phoneNumber: string, code: string }
- Returns: string (JWT token)

// Logout
LOGOUT
- Variables: { sessionToken: string }
- Returns: boolean
```

### Admin Mutations (Require Auth)

```typescript
// Create listing
CREATE_LISTING
- Variables: { input: CreateListingInput }
- Returns: Listing
- Requires: Admin JWT token

// Update listing status
UPDATE_LISTING_STATUS
- Variables: { listingId: string, status: string }
- Returns: Listing
- Requires: Admin JWT token
```

See `packages/web-next/lib/graphql.ts` for complete list.

## TypeScript Types

All API types are defined in `packages/web-next/lib/types.ts`:

```typescript
import type {
  Organization,
  OrganizationMatch,
  Listing,
  Post,
  SearchOrganizationsResult,
  CreateListingInput,
} from "@/lib/types";
```

## Environment Variables

### Frontend (Next.js)

```bash
# packages/web-next/.env.local
NEXT_PUBLIC_API_URL=http://localhost:8080/graphql
```

**Production:**
```bash
NEXT_PUBLIC_API_URL=https://api.mndigitalaid.org/graphql
```

### Backend (Rust)

```bash
# packages/server/.env
ALLOWED_ORIGINS=http://localhost:3000,http://localhost:19006,http://localhost:8081

# Production
ALLOWED_ORIGINS=https://mndigitalaid.org,https://www.mndigitalaid.org
```

## Troubleshooting

### CORS Error

**Error:** `Access to fetch has been blocked by CORS policy`

**Solutions:**
1. Check Rust server is running: `curl http://localhost:8080/health`
2. Verify `ALLOWED_ORIGINS` in `packages/server/.env` includes your frontend URL
3. Restart Rust server after changing `.env`
4. Check browser console for exact origin being blocked

### 401 Unauthorized

**Error:** GraphQL returns 401

**Solution:**
- Admin mutations require JWT token
- Get token via authentication flow:
  1. Call `sendVerificationCode(phoneNumber)`
  2. Call `verifyCode(phoneNumber, code)`
  3. Store returned JWT token
  4. Pass token in requests: `graphqlFetchClient(query, vars, token)`

### Network Error

**Error:** `Failed to fetch` or `NetworkError`

**Solutions:**
1. Check backend is running: `curl http://localhost:8080/health`
2. Check frontend API URL: `console.log(process.env.NEXT_PUBLIC_API_URL)`
3. Check network tab in browser DevTools
4. Verify no firewall blocking localhost:8080

### GraphQL Errors

**Error:** GraphQL returns errors array

**Example:**
```json
{
  "errors": [
    { "message": "Organization not found" }
  ]
}
```

**Solution:**
- These are application errors, not connection errors
- Check error message for details
- Verify query variables are correct
- Check if resource exists in database

## Testing API

### GraphQL Playground

Development only: http://localhost:8080/graphql

Try this query:
```graphql
query SearchOrganizations {
  searchOrganizationsSemantic(query: "food banks", limit: 5) {
    organization {
      id
      name
      description
      website
    }
    similarityScore
  }
}
```

### curl Commands

```bash
# Test health endpoint
curl http://localhost:8080/health

# Test GraphQL query
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{
    "query": "{ organizations { id name } }"
  }'

# Test with authentication
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your-jwt-token>" \
  -d '{
    "query": "mutation { createListing(...) { id } }"
  }'
```

### Insomnia/Postman

1. Create new request
2. Method: POST
3. URL: `http://localhost:8080/graphql`
4. Headers:
   - `Content-Type: application/json`
   - `Authorization: Bearer <token>` (for admin mutations)
5. Body (JSON):
```json
{
  "query": "{ organizations { id name } }",
  "variables": {}
}
```

## Production Deployment

### CORS Configuration

**Critical:** Update `ALLOWED_ORIGINS` before deploying:

```bash
# packages/server/.env (production)
ALLOWED_ORIGINS=https://mndigitalaid.org,https://www.mndigitalaid.org
```

**Never** use:
- `*` (allows all origins - security risk)
- `http://` in production (requires HTTPS)
- Localhost origins in production

### Verify CORS in Production

```bash
# Test preflight request
curl -X OPTIONS https://api.mndigitalaid.org/graphql \
  -H "Origin: https://mndigitalaid.org" \
  -H "Access-Control-Request-Method: POST" \
  -H "Access-Control-Request-Headers: Content-Type" \
  -v

# Look for these headers:
# Access-Control-Allow-Origin: https://mndigitalaid.org
# Access-Control-Allow-Methods: GET, POST
# Access-Control-Allow-Headers: authorization, content-type
```

### Rate Limiting

The API has rate limiting enabled:
- **Rate:** 10 requests/second per IP
- **Burst:** 20 requests
- **Scope:** Per IP address (uses X-Forwarded-For)

If you hit rate limits, you'll receive `429 Too Many Requests`.

### Health Checks

Monitor API health:

```bash
# Should return: { "status": "healthy" }
curl https://api.mndigitalaid.org/health
```

## Best Practices

### 1. Error Handling

Always handle errors in try-catch:

```typescript
try {
  const data = await graphqlFetch(query, variables);
  // Handle success
} catch (error) {
  console.error("API error:", error);
  // Show user-friendly error message
}
```

### 2. Loading States

Show loading indicators:

```typescript
const [loading, setLoading] = useState(false);

const fetchData = async () => {
  setLoading(true);
  try {
    const data = await graphqlFetch(...);
    // Handle data
  } finally {
    setLoading(false);
  }
};
```

### 3. Caching

Use Next.js caching for server components:

```typescript
// Cache for 5 minutes
graphqlFetch(query, vars, { revalidate: 300 });

// No cache (always fetch fresh)
graphqlFetch(query, vars, { revalidate: 0 });

// Cache indefinitely
graphqlFetch(query, vars, { revalidate: false });
```

### 4. Type Safety

Always use TypeScript types:

```typescript
import type { SearchOrganizationsResult } from "@/lib/types";

const data = await graphqlFetch<SearchOrganizationsResult>(
  SEARCH_ORGANIZATIONS,
  { query: "legal aid" }
);

// TypeScript knows the shape of data
data.searchOrganizationsSemantic.forEach(...);
```

### 5. Authentication Tokens

Store tokens securely:

```typescript
// Client-side only (never in server components)
const token = localStorage.getItem("auth_token");

// Include in requests
const data = await graphqlFetchClient(query, vars, token);

// Clear on logout
localStorage.removeItem("auth_token");
```

## Support

For API integration issues:

1. Check this guide
2. Test API with curl/Postman first
3. Check browser Network tab for actual requests
4. Review `packages/server/src/server/graphql/schema.rs` for available queries
5. Check server logs: `RUST_LOG=debug cargo run`

## Related Documentation

- [Deployment Guide](../DEPLOYMENT.md) - Full deployment instructions
- [GraphQL Schema](../packages/server/src/server/graphql/schema.rs) - Available queries/mutations
- [Server Configuration](../packages/server/src/config.rs) - Environment variables
