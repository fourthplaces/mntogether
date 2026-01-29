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

### Prerequisites

- Node.js 22+
- Yarn 4.1+

### Setup

```bash
# Install dependencies
yarn install

# Set up environment variables
cp .env.local.example .env.local
# Edit .env.local with your API URL
```

### Environment Variables

```env
NEXT_PUBLIC_API_URL=http://localhost:8080/graphql
```

### Run Development Server

```bash
yarn dev
```

Open [http://localhost:3000](http://localhost:3000) to view the app.

### Build

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

## GraphQL Queries

The app uses the following GraphQL queries:

### Search Organizations

```graphql
query SearchOrganizations($query: String!, $limit: Int) {
  searchOrganizationsSemantic(query: $query, limit: $limit) {
    organization {
      id
      name
      description
      url
    }
    similarityScore
  }
}
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
