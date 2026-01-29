# MN Digital Aid - Web App

Unified React + Vite SPA that includes both public-facing pages and admin dashboard.

## Features

### Public Pages
- **Home**: Display published needs/posts with analytics tracking
- **Submit Resource**: Public form to submit resource URLs for scraping

### Admin Dashboard (Protected)
- **Authentication**: Phone/email verification with JWT
- **Need Approval Queue**: Review and approve/reject pending needs
- **Resources Management**: Manage organization sources and trigger scraping
- **Organization Detail**: Configure scrape URLs for targeted crawling
- **Resource Detail**: View and manage needs by organization

## Tech Stack

- React 18.3
- Vite 6.2
- TypeScript 5.6
- Tailwind CSS 3.4
- Apollo Client (GraphQL)
- React Router 7.1
- React Markdown 9.0

## Development

### Prerequisites

- Node.js 22+
- Yarn 4.1+
- Running API server (via Docker Compose or locally)

### Setup

```bash
# Install dependencies
yarn install

# Start development server
yarn dev
```

The app runs on **port 3001** and proxies `/graphql` requests to `http://localhost:8080`.

Access:
- **Public site**: http://localhost:3001
- **Admin dashboard**: http://localhost:3001/admin

## Routes

### Public Routes
- `/` - Home page with published posts
- `/submit` - Submit resource form

### Admin Routes (Protected)
- `/admin/login` - Admin login (phone/email verification)
- `/admin` - Approval queue
- `/admin/resources` - Organization sources list
- `/admin/resources/:sourceId` - Resource detail
- `/admin/organizations/:sourceId` - Organization scrape URL management

## Authentication

Admin routes require authentication via JWT token stored in localStorage (`admin_jwt_token`).

**Login methods:**
- Phone number verification (SMS via Twilio)
- Email verification

**To become an admin:**
Add your phone number to `ADMIN_IDENTIFIERS` in the server's `.env` file.

## Architecture

```
src/
├── App.tsx                    # Main app with routing
├── main.tsx                   # Entry point
├── contexts/
│   └── AuthContext.tsx        # JWT auth management
├── pages/
│   ├── Home.tsx               # Public: published posts
│   ├── SubmitResource.tsx     # Public: submit URL
│   └── admin/                 # Admin pages (protected)
│       ├── Login.tsx
│       ├── NeedApprovalQueue.tsx
│       ├── Resources.tsx
│       ├── ResourceDetail.tsx
│       └── OrganizationDetail.tsx
├── components/
│   └── PostCard.tsx           # Display post card with tracking
└── graphql/
    ├── client.ts              # Apollo client with auth link
    ├── queries.ts             # All GraphQL queries
    └── mutations.ts           # All GraphQL mutations
```

## Building

```bash
# Production build
yarn build

# Preview production build
yarn preview
```

## Deployment

The app is deployed to CloudFront + S3 via GitHub Actions:
- Push to `main` → deploys to prod (https://app.mndigitalaid.org)
- Push to `dev` → deploys to dev (https://app.dev.mndigitalaid.org)

See [infra/packages/web-app/](../../infra/packages/web-app/) for infrastructure details.

## GraphQL Operations

### Public Queries
- `GET_PUBLISHED_POSTS` - Fetch published posts

### Public Mutations
- `SUBMIT_NEED` - Submit a need
- `SUBMIT_RESOURCE_LINK` - Submit URL for scraping
- `TRACK_POST_VIEW` - Analytics: track post view
- `TRACK_POST_CLICK` - Analytics: track post click

### Admin Queries (Requires Auth)
- `GET_PENDING_NEEDS` - Fetch needs awaiting approval
- `GET_ACTIVE_NEEDS` - Fetch active needs
- `GET_NEED_DETAIL` - Single need details
- `GET_ORGANIZATION_SOURCES` - List all organizations
- `GET_ORGANIZATION_SOURCE_NEEDS` - Needs for specific org
- `GET_POSTS_FOR_NEED` - Posts published for a need

### Admin Mutations (Requires Auth)
- `APPROVE_NEED` - Approve pending need
- `EDIT_AND_APPROVE_NEED` - Edit and approve in one action
- `REJECT_NEED` - Reject need with reason
- `SEND_VERIFICATION_CODE` - Send SMS/email verification code
- `VERIFY_CODE` - Verify code and get JWT token
- `SCRAPE_ORGANIZATION` - Trigger scraping job for organization
- `EXPIRE_POST` - Expire published post
- `ARCHIVE_POST` - Archive post
- `DELETE_NEED` - Delete need
- `ADD_ORGANIZATION_SCRAPE_URL` - Add specific URL to scrape
- `REMOVE_ORGANIZATION_SCRAPE_URL` - Remove scrape URL

## Styling

- **Public pages**: Gray/blue color scheme (cool professional tones)
- **Admin pages**: Amber/stone color scheme (warm earth tones)

Both use Tailwind CSS utility classes.

## PostCard Component

Displays need/post with:
- Organization name
- Title and description
- Urgency badge (color-coded)
- Contact information
- Automatic view/click tracking

## Contributing

See the main [project README](../../README.md) for contribution guidelines.
