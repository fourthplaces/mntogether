# Minnesota Digital Aid

A privacy-first volunteer matching platform that connects community members with immigrant resource organizations using AI-powered semantic search and location-based filtering.

## Overview

This platform helps volunteers discover opportunities at immigrant resource organizations in Minnesota. It features:

- **Privacy-First Design**: No PII storage, only coarse location data and Expo push tokens
- **Text-First Architecture**: Searchable text as source of truth for anti-fragile evolution
- **AI-Powered Matching**: GPT-4o extracts needs, embeddings enable semantic search
- **Event-Driven Architecture**: Built with seesaw-rs for clean separation of concerns
- **Location-Based Filtering**: 30km radius matching using PostGIS distance calculations
- **Smart Notifications**: Weekly throttling (max 3) with AI relevance checking

## Quick Start

### Prerequisites

- Rust 1.70+ and Cargo ([Install from rustup.rs](https://rustup.rs/))
- Docker and Docker Compose ([Install Docker Desktop](https://www.docker.com/products/docker-desktop/))
- PostgreSQL with pgvector extension
- API keys: OpenAI, Firecrawl, Twilio (SMS auth)

### 🚀 One-Command Setup

The easiest way to get started:

```bash
./dev.sh
```

This single entry point will:
1. Install all dependencies automatically
2. Build the project
3. Present an interactive menu for:
   - Starting the mobile app (Expo)
   - Managing Docker services
   - Viewing logs

See [DEV_CLI.md](DEV_CLI.md) for complete documentation.

### Environment Variables

Before starting, create your environment file:

```bash
cd packages/server
cp .env.example .env
# Edit .env and add your API keys
```

Required keys:
```env
# Core services
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mndigitalaid
REDIS_URL=redis://localhost:6379

# AI/ML
OPENAI_API_KEY=sk-...                    # Required: GPT-4o + embeddings

# Web scraping
FIRECRAWL_API_KEY=fc-...                 # Required: Source scraping
TAVILY_API_KEY=tvly-...                  # Optional: Search discovery

# SMS authentication
TWILIO_ACCOUNT_SID=AC...                 # Required: SMS verification
TWILIO_AUTH_TOKEN=...                    # Required
TWILIO_VERIFY_SERVICE_SID=VA...          # Required

# JWT authentication
JWT_SECRET=...                           # Required: 32+ byte secret (generate with: openssl rand -base64 32)
JWT_ISSUER=mndigitalaid                  # Optional: JWT issuer identifier

# Push notifications
EXPO_ACCESS_TOKEN=...                    # Optional: Higher rate limits

# Server
PORT=8080
RUST_LOG=info,server_core=debug
```

### Manual Setup (Alternative)

If you prefer manual control:

#### Backend Setup (Rust + GraphQL)

1. **Navigate to server package**:
   ```bash
   cd packages/server
   ```

2. **Start all services**:
   ```bash
   make up
   # or: docker-compose up -d
   ```

3. **Run database migrations**:
   ```bash
   make migrate
   # or: docker-compose exec api cargo sqlx migrate run
   ```

4. **Prepare SQLx offline data** (required for compilation):
   ```bash
   cargo sqlx prepare --workspace
   # This creates .sqlx/ with cached query data for offline compilation
   ```

5. **Build the server**:
   ```bash
   cargo build
   # or from packages/server: make build
   ```

6. **Check logs**:
   ```bash
   make logs
   # or: docker-compose logs -f
   ```

#### Frontend Setup

##### Next.js Public Site (SSR)
The Next.js app runs automatically with docker-compose and includes hot-reload:
```bash
# Already running with docker-compose up
# Access: http://localhost:3000
```

Or run standalone:
```bash
cd packages/web-next
yarn install
yarn dev
# Access: http://localhost:3000
```

##### Web App (React + Vite)
The unified web app includes both public pages and admin dashboard:
```bash
cd packages/web-app
yarn install
yarn dev
# Access: http://localhost:3001
# Admin: http://localhost:3001/admin
```

##### Expo App (React Native)
```bash
cd packages/app
npm install
npm start
# Press 'a' for Android, 'i' for iOS, 'w' for web
```

### Available Services

- **PostgreSQL**: `localhost:5432` (with pgvector extension)
- **Redis**: `localhost:6379`
- **API**: `localhost:8080`
- **GraphiQL Playground**: `localhost:8080/graphql`
- **Health Check**: `localhost:8080/health`
- **Next.js Public Site**: `localhost:3000` (SSR with hot-reload)
- **Web App (SPA)**: `localhost:3001` (Public + Admin with hot-reload)
- **Admin Dashboard**: `localhost:3001/admin`

### Seeding Data

After migrations, seed the database with real organizations:

```bash
cd packages/server
cargo run --bin seed_organizations
```

This imports 50+ immigrant resource organizations from JSON with AI-powered tag extraction.

### Development Commands

```bash
# From packages/server/
make help          # Show all available commands
make up            # Start services
make down          # Stop services
make logs          # View logs
make migrate       # Run migrations
make test          # Run tests
make build         # Build Rust project
make check         # Fast compile check
make shell         # Open shell in API container
make db-shell      # Open PostgreSQL shell
make redis-cli     # Open Redis CLI
```

## Workspace Packages

This project uses Cargo workspaces with the following packages:

- **server**: Main GraphQL API server with event-driven architecture
- **seesaw-rs**: Event-driven framework (events, commands, machines, effects)
- **twilio-rs**: Twilio SMS authentication client wrapper
- **dev-cli**: Interactive CLI for managing development tasks

## Project Structure

```
mndigitalaid/
├── Cargo.toml                    # Workspace root
├── packages/
│   ├── server/                   # Backend (Rust + GraphQL + seesaw-rs)
│   │   ├── src/
│   │   │   ├── common/           # Shared utilities
│   │   │   │   └── utils/        # geocoding, embeddings, expo client
│   │   │   ├── domains/          # Business domains (event-driven)
│   │   │   │   ├── organization/ # Need discovery & approval
│   │   │   │   │   ├── models/   # SQL persistence
│   │   │   │   │   ├── data/     # GraphQL types
│   │   │   │   │   ├── events/   # Domain events
│   │   │   │   │   ├── commands/ # Intent definitions
│   │   │   │   │   ├── machines/ # State machines
│   │   │   │   │   ├── effects/  # IO handlers
│   │   │   │   │   └── edges/    # GraphQL resolvers
│   │   │   │   ├── member/       # Volunteer registration
│   │   │   │   └── matching/     # Semantic search + notifications
│   │   │   ├── kernel/           # Core infrastructure
│   │   │   │   ├── jobs/         # Background job queue
│   │   │   │   └── scheduled_tasks.rs  # Cron jobs
│   │   │   └── server/           # HTTP server + GraphQL
│   │   ├── migrations/           # PostgreSQL migrations
│   │   ├── tests/                # Integration tests
│   │   ├── Cargo.toml
│   │   ├── docker-compose.yml
│   │   └── Makefile
│   ├── seesaw-rs/                # Event-driven framework
│   ├── twilio-rs/                # SMS authentication client
│   ├── dev-cli/                  # Interactive development CLI
│   ├── admin-spa/                # Admin panel (React + Vite)
│   └── expo-app/                 # Mobile app (React Native)
└── docs/                         # Documentation
```

## Development Status

### ✅ MVP COMPLETE - SHIPPABLE

All core features implemented and ready for production deployment:

#### Organization Domain
- ✅ Organization CRUD with tag system (services, languages, communities)
- ✅ Web scraping with Firecrawl API
- ✅ AI need extraction using GPT-4o via rig.rs
- ✅ Content hash-based duplicate detection
- ✅ Human-in-the-loop approval workflow
- ✅ Post creation for temporal announcements
- ✅ Complete GraphQL API

#### Member Domain
- ✅ Privacy-first registration (coarse location, no PII)
- ✅ Text-first profile (searchable_text as source of truth)
- ✅ Auto-geocoding (city/state → lat/lng)
- ✅ Embedding generation (text-embedding-3-small)
- ✅ Weekly notification throttling (max 3/week)
- ✅ GraphQL API: register, query members

#### Matching Domain
- ✅ Distance-filtered vector search (30km radius)
- ✅ Embedding similarity ranking
- ✅ AI relevance checking
- ✅ Expo push notification delivery
- ✅ Atomic throttle checking
- ✅ Notification tracking and analytics

#### Infrastructure
- ✅ Event-driven architecture (seesaw-rs)
- ✅ Background job queue (Postgres-based)
- ✅ Scheduled tasks (hourly scraping, weekly reset)
- ✅ Integration test harness
- ✅ Docker Compose setup

See [MVP_COMPLETE.md](MVP_COMPLETE.md) for full details.

## Architecture Highlights

### Event-Driven (seesaw-rs)
Clean separation of concerns following the seesaw pattern:
```
Request Event → Machine (decide) → Command → Effect (IO) → Fact Event
```
- **Events**: Immutable facts about what happened
- **Commands**: Intent to perform an action
- **Machines**: Pure decision logic (no IO)
- **Effects**: Stateless IO handlers
- **Edges**: Thin GraphQL resolvers that dispatch requests

### Domain Structure
Each domain follows strict layering:
- `models/`: SQL queries only (no business logic)
- `data/`: GraphQL types with lazy resolvers
- `events/`: Domain event definitions
- `commands/`: Command definitions with execution modes
- `machines/`: State machines for decision logic
- `effects/`: IO implementations (API calls, DB writes)
- `edges/`: GraphQL query/mutation resolvers

### Key Principles
- **Privacy-First**: Coarse coordinates (city-level), no PII, only Expo tokens
- **Text-First**: `searchable_text` as source of truth for anti-fragile evolution
- **Human-in-the-Loop**: AI extracts needs → Admin approves → Matching triggered
- **Location as Filter**: Distance filtering (30km) before semantic ranking
- **Generous Matching**: Bias toward recall, not precision
- **Content Hash Sync**: Detect new/changed/disappeared needs automatically

## Technology Stack

### Backend
- **Rust**: Type-safe, high-performance systems language
- **seesaw-rs**: Custom event-driven framework for clean architecture
- **Axum**: Modern async web framework
- **Juniper**: GraphQL server implementation
- **SQLx**: Compile-time checked SQL queries
- **PostgreSQL + pgvector**: Vector similarity search
- **Redis**: Job queue and caching

### AI/ML
- **OpenAI GPT-4o**: Need extraction and relevance checking
- **OpenAI text-embedding-3-small**: 1536-dimensional embeddings
- **rig.rs**: Type-safe AI/LLM integration framework

### External Services
- **Firecrawl**: Headless browser for web scraping
- **Nominatim (OpenStreetMap)**: Free geocoding service
- **Expo**: Push notification delivery
- **Twilio Verify**: SMS authentication

### Frontend (Planned)
- **React + Vite**: Admin approval dashboard
- **React Native + Expo**: Mobile volunteer app

## Running the Server

### Development Mode
```bash
cd packages/server
cargo run --bin server
# Server starts at http://localhost:8080
# GraphQL playground at http://localhost:8080/graphql
```

### Production Build
```bash
cd packages/server
cargo build --release
./target/release/server
```

### Background Jobs
The server automatically runs:
- **Hourly**: Organization source scraping
- **Weekly (Monday midnight)**: Reset notification throttles

## Performance Characteristics

### Query Times (Expected)
- Member registration: ~500ms (includes geocoding)
- Embedding generation: ~200ms per text
- Vector search: ~10-20ms (with indexes)
- AI relevance check: ~200ms per candidate
- Expo notification: ~100ms per push
- Full matching pipeline: ~2-3s per approved need

### Scalability
- **Current**: Good for <10K members
- **With indexes**: Good for <100K members
- **For >100K**: Consider PostGIS + spatial indexes

### Database Indexes
- IVFFlat indexes on embedding vectors for fast similarity search
- Spatial indexes on latitude/longitude for distance queries
- Hash indexes on UUIDs and tokens for fast lookups

## API Examples

### Register a Member
```graphql
mutation {
  registerMember(
    expoPushToken: "ExponentPushToken[xyz]"
    searchableText: "Can drive, speak Spanish, interested in food assistance"
    city: "Minneapolis"
    state: "MN"
  ) {
    id
    locationName
    latitude
    longitude
  }
}
```

### Approve a Need (Triggers Matching)
```graphql
mutation {
  approveNeed(needId: "uuid-here") {
    id
    status
  }
}
```

This automatically:
1. Generates embedding for the need
2. Searches for members within 30km
3. Ranks by semantic similarity
4. Checks AI relevance
5. Sends push notifications
6. Records in notifications table

### Query Organizations
```graphql
query {
  searchOrganizations(query: "food assistance") {
    id
    name
    tags {
      kind
      value
    }
    sources {
      sourceUrl
    }
  }
}
```

## Testing

### Run All Tests
```bash
cd packages/server
cargo test
```

### Run Specific Test File
```bash
cargo test --test organization_needs_tests
cargo test --test content_hash_tests
```

### Integration Tests
The project includes integration tests with:
- PostgreSQL test containers
- Redis test containers
- Test fixtures for organizations and members
- GraphQL query testing harness

## Deployment

### Prerequisites
- PostgreSQL 14+ with pgvector extension
- Redis 6+
- Valid API keys for OpenAI, Firecrawl, Twilio

### Environment Setup
1. Copy `.env.example` to `.env`
2. Fill in all required API keys
3. Set `DATABASE_URL` to production database
4. Set `REDIS_URL` to production Redis instance

### Database Setup
```bash
# Run migrations
sqlx migrate run

# Seed organizations (optional)
cargo run --bin seed_organizations
```

### Start Server
```bash
cargo run --release --bin server
```

### Health Check
```bash
curl http://localhost:8080/health
# Should return: {"status":"healthy"}
```

## Known Limitations

1. **AI Relevance Check**: Currently uses similarity threshold to save costs
2. **Geocoding**: Free tier (Nominatim) - consider paid service for production
3. **No admin UI**: GraphQL only, frontend needed for approval workflow
4. **No notification preferences**: All members get same notification types
5. **No retry logic**: Expo notifications don't retry on failure

## Learn More

- [MVP_COMPLETE.md](MVP_COMPLETE.md) - Complete feature list and testing checklist
- [REFACTORING_COMPLETE.md](REFACTORING_COMPLETE.md) - Event-driven architecture details
- [MATCHING_IMPLEMENTATION.md](MATCHING_IMPLEMENTATION.md) - Location-based matching system
- [DEV_CLI.md](DEV_CLI.md) - Interactive development CLI documentation

## License

MIT
