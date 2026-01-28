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

##### Admin UI (React + Vite)
```bash
cd packages/admin-spa
npm install
npm run dev
# Access: http://localhost:3000
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
- **Admin UI**: `localhost:3000`

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

## Project Structure

```
mndigitalaid/
├── Cargo.toml            # Workspace root
├── packages/
│   ├── server/           # Backend (Rust + GraphQL)
│   │   ├── src/
│   │   │   ├── common/           # Shared utilities
│   │   │   ├── domains/          # Business domains
│   │   │   │   └── organization/ # Need discovery (SPIKE 1)
│   │   │   ├── kernel/           # Core infrastructure
│   │   │   └── server/           # HTTP server + routes
│   │   ├── migrations/           # Database migrations
│   │   ├── tests/                # Integration tests
│   │   ├── Cargo.toml
│   │   ├── docker-compose.yml
│   │   └── Makefile
│   ├── admin-spa/        # Admin panel (React + Vite)
│   │   ├── src/
│   │   │   ├── pages/            # NeedApprovalQueue
│   │   │   └── graphql/          # Queries + mutations
│   │   └── package.json
│   └── expo-app/         # Public volunteer app (React Native)
│       ├── src/
│       │   ├── screens/          # NeedList + NeedDetail
│       │   └── graphql/          # Queries + mutations
│       └── package.json
└── docs/                 # Documentation + plans
```

## Development Roadmap

### SPIKE 1: Need Discovery Pipeline ✅ COMPLETE
- ✅ Database migrations (PostgreSQL + pgvector)
- ✅ Firecrawl scraper client
- ✅ AI need extraction (rig.rs + GPT-4o)
- ✅ Content hash sync logic
- ✅ GraphQL API (queries + mutations)
- ✅ Admin approval UI (React)
- ✅ Expo app screens (list + detail)
- ✅ User-submitted needs with IP geolocation
- ✅ Integration tests with test harness
- ✅ Human-in-the-loop approval workflow

See [docs/SPIKE_1_COMPLETE.md](docs/SPIKE_1_COMPLETE.md) for full details.

### SPIKE 2: Volunteer Intake (Next)
- Bell icon registration flow
- Quick options (checkboxes)
- Text-first form
- Expo push token collection

### SPIKE 3: AI Chat (Optional)
- Real-time chat UI with Redis pub/sub
- GraphQL subscriptions

## Architecture Highlights

- **Privacy-First**: Zero PII stored, only expo_push_token
- **Text-First**: searchable_text as source of truth
- **Human-in-the-Loop**: AI extracts needs → Admin approves → Users see
- **Content Hash Sync**: Detect new/changed/disappeared needs
- **Event-Driven**: seesaw-rs (events → machines → commands → effects)

## Learn More

See [docs/plans/2026-01-27-mvp-execution-plan.md](docs/plans/2026-01-27-mvp-execution-plan.md) for the full execution plan.
