# MN Digital Aid - Next.js Public Site

Server-side rendered Next.js application for the public-facing MN Digital Aid website. Built with Next.js 15, TypeScript, and Tailwind CSS.

## Features

- **Server-Side Rendering**: SEO-optimized with SSR for crawlability
- **GraphQL Integration**: Fetches data from the Rust GraphQL API
- **Semantic Search**: Search organizations using AI-powered semantic matching
- **Incremental Static Regeneration**: Pages revalidate every 60 seconds
- **Standalone Output**: Optimized Docker deployment

## Tech Stack

- Next.js 15 (App Router)
- TypeScript
- Tailwind CSS
- GraphQL client (fetch-based)

## Development

### Option 1: Docker Compose (Recommended)

Run the entire stack (PostgreSQL, Redis, API, Next.js) with Docker Compose:

```bash
# From the server directory
cd ../server

# Start all services
docker-compose up

# Or run in background
docker-compose up -d

# View logs
docker-compose logs -f web-next

# Stop services
docker-compose down
```

The Next.js app will be available at [http://localhost:3000](http://localhost:3000).

Hot-reload is enabled - any changes to the source code will automatically refresh the browser.

### Option 2: Local Development

If you prefer to run Next.js locally without Docker:

**Prerequisites:**
- Node.js 22+
- Yarn 4.1+
- Running API server (via Docker Compose or locally)

**Setup:**

```bash
# Install dependencies
yarn install

# Set up environment variables
cp .env.local.example .env.local
# Edit .env.local with your API URL
```

**Environment Variables:**

```env
NEXT_PUBLIC_API_URL=http://localhost:8080/graphql
```

**Run Development Server:**

```bash
yarn dev
```

Open [http://localhost:3000](http://localhost:3000) to view the app.

**Build:**

```bash
yarn build
yarn start
```

## Docker

Build and run with Docker:

```bash
# Build image
docker build -t mndigitalaid-web-next .

# Run container
docker run -p 3000:3000 \
  -e NEXT_PUBLIC_API_URL=https://api.mndigitalaid.org/graphql \
  mndigitalaid-web-next
```

## Deployment

The app is automatically deployed to AWS ECS Fargate via GitHub Actions:

- Push to `main` → deploys to prod (https://www.mndigitalaid.org)
- Push to `dev` → deploys to dev (https://www.dev.mndigitalaid.org)

### Manual Deployment

```bash
# Build and push Docker image
docker build -t <ecr-repo>:tag .
docker push <ecr-repo>:tag

# Deploy with Pulumi
cd ../../infra/packages/web-next
pulumi config set imageTag <tag>
pulumi up --yes
```

## Project Structure

```
packages/web-next/
├── app/                    # Next.js App Router
│   ├── (public)/          # Public pages
│   │   ├── page.tsx       # Home page
│   │   └── search/        # Search page
│   │       └── page.tsx
│   └── layout.tsx         # Root layout
├── components/            # React components
├── lib/                   # Utilities
│   └── graphql.ts         # GraphQL client
├── next.config.ts         # Next.js configuration
├── tailwind.config.ts     # Tailwind configuration
└── Dockerfile             # Docker build
```

## API Integration

### GraphQL Client

The app includes a type-safe GraphQL client at `lib/graphql.ts` with two functions:

- **`graphqlFetch()`** - For Server Components (SSR)
- **`graphqlFetchClient()`** - For Client Components

### Server Components (Recommended)

```typescript
import { graphqlFetch, SEARCH_ORGANIZATIONS } from "@/lib/graphql";
import type { SearchOrganizationsResult } from "@/lib/types";

export default async function Page() {
  const data = await graphqlFetch<SearchOrganizationsResult>(
    SEARCH_ORGANIZATIONS,
    { query: "food assistance", limit: 10 },
    { revalidate: 60 } // Cache for 60 seconds
  );

  return (
    <div>
      {data.searchOrganizationsSemantic.map((match) => (
        <div key={match.organization.id}>
          {match.organization.name}
        </div>
      ))}
    </div>
  );
}
```

### Client Components

```typescript
"use client";

import { useState } from "react";
import { graphqlFetchClient, SEARCH_ORGANIZATIONS } from "@/lib/graphql";
import type { SearchOrganizationsResult } from "@/lib/types";

export function SearchComponent() {
  const [results, setResults] = useState(null);

  const search = async (query: string) => {
    const data = await graphqlFetchClient<SearchOrganizationsResult>(
      SEARCH_ORGANIZATIONS,
      { query, limit: 10 }
    );
    setResults(data);
  };

  return <input onChange={(e) => search(e.target.value)} />;
}
```

See `app/(public)/search/SearchClient.example.tsx` for a complete example.

### Available Queries & Mutations

All queries and mutations are defined in `lib/graphql.ts`:

**Organizations:**
- `SEARCH_ORGANIZATIONS` - AI semantic search
- `GET_ORGANIZATIONS` - All verified organizations
- `GET_ORGANIZATION` - Get by ID

**Listings:**
- `GET_LISTING` - Get listing by ID
- `GET_LISTINGS_BY_TYPE` - Filter by type (service, opportunity, business)
- `GET_LISTINGS_BY_CATEGORY` - Filter by category
- `SEARCH_LISTINGS` - Advanced search with filters

**Posts:**
- `GET_PUBLISHED_POSTS` - All published posts
- `GET_POST` - Get by ID

**Auth:**
- `SEND_VERIFICATION_CODE` - SMS verification
- `VERIFY_CODE` - Get JWT token
- `LOGOUT` - End session

**Public Mutations:**
- `SUBMIT_RESOURCE_LINK` - Submit URL for scraping
- `TRACK_POST_VIEW` - Analytics
- `TRACK_POST_CLICK` - Analytics

**Admin Mutations:**
- `CREATE_LISTING` - Create new listing
- `UPDATE_LISTING_STATUS` - Update status
- `UPDATE_LISTING_CAPACITY` - Update capacity

### TypeScript Types

All API types are in `lib/types.ts`:

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

### CORS

CORS is automatically configured for development:
- `http://localhost:3000` (Next.js)
- `http://localhost:19006` (Expo web)
- `http://localhost:8081` (React Native)

For production, set `ALLOWED_ORIGINS` in the Rust server's `.env` file.

### Testing the API

**GraphQL Playground (Development):**
http://localhost:8080/graphql

**curl:**
```bash
curl http://localhost:8080/health
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ organizations { id name } }"}'
```

**Browser Console:**
```javascript
fetch('http://localhost:8080/graphql', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    query: '{ organizations { id name } }'
  })
}).then(r => r.json()).then(console.log)
```

## Routes

- `/` - Home page with service cards
- `/search?q=query` - Search results page with semantic matching

## Performance

- ISR revalidation: 60 seconds
- Standalone output for minimal Docker image size
- Next.js Image optimization with AVIF/WebP support

## Monitoring

View logs in CloudWatch:

```bash
aws logs tail /mndigitalaid/dev/web-next --follow
```

## Contributing

See the main [project README](../../README.md) for contribution guidelines.
