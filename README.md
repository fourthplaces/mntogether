# Emergency Resource Aggregator

A privacy-first platform that matches volunteers with organization needs using AI.

## Quick Start

### Prerequisites

- Rust 1.70+ and Cargo
- Docker and Docker Compose
- Node.js 18+ (for frontend development)
- API keys for external services

### Backend Setup (Rust + GraphQL)

1. **Navigate to server package**:
   ```bash
   cd packages/server
   ```

2. **Copy environment variables**:
   ```bash
   cp .env.example .env
   ```

3. **Edit `.env` and add your API keys**:
   ```
   OPENAI_API_KEY=sk-...
   FIRECRAWL_API_KEY=fc-...
   ```

4. **Start all services**:
   ```bash
   make up
   # or: docker-compose up -d
   ```

5. **Run database migrations**:
   ```bash
   make migrate
   # or: docker-compose exec api cargo sqlx migrate run
   ```

6. **Prepare SQLx offline data** (required for compilation):
   ```bash
   cargo sqlx prepare --workspace
   # This creates .sqlx/ with cached query data for offline compilation
   ```

7. **Build the server**:
   ```bash
   cargo build
   # or from packages/server: make build
   ```

8. **Check logs**:
   ```bash
   make logs
   # or: docker-compose logs -f
   ```

### Frontend Setup

#### Admin UI (React + Vite)
```bash
cd packages/admin-spa
npm install
npm run dev
# Access: http://localhost:3000
```

#### Expo App (React Native)
```bash
cd packages/expo-app
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
