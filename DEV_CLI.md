# Development CLI

Single entry point for working with the Minnesota Digital Aid application.

## Quick Start

After cloning the repository, simply run:

```bash
./dev.sh
```

The script will:
1. Check for required dependencies (Rust, Docker, Node.js)
2. Build the dev CLI if needed
3. On first run, install all project dependencies
4. Present an interactive menu for common development tasks

## Requirements

The following must be installed on your system:
- **Rust** (cargo) - [Install from rustup.rs](https://rustup.rs/)
- **Docker** - [Install Docker Desktop](https://www.docker.com/products/docker-desktop/)
- **Node.js** (v18+) - [Install from nodejs.org](https://nodejs.org/)
- **npm** - Included with Node.js

## Interactive Menu

The CLI provides the following options:

### 📱 Start mobile (Expo)
Starts the Expo development server for the mobile app. You can then:
- Press `i` to open iOS Simulator
- Press `a` to open Android Emulator
- Scan QR code with Expo Go app on your device

### 🐳 Docker start
Starts all Docker services in detached mode:
- PostgreSQL database (port 5432)
- Redis (port 6379)
- API server (port 8080)

### 🔄 Docker restart
Restarts all running Docker services without rebuilding.

### 🔨 Docker rebuild
Rebuilds Docker images from scratch and starts the services.
Use this when:
- You've modified the Dockerfile
- You've added new dependencies
- You want a clean build

### 📋 Follow docker logs
Tails the logs from all Docker services.
Press `Ctrl+C` to stop following.

### 🛑 Exit
Exits the CLI.

## First Time Setup

On first run, the CLI will:
1. Check for required tools (cargo, docker, node, npm)
2. Install Expo CLI globally if not present
3. Install app dependencies (`npm install` in packages/app)
4. Build the Rust workspace

## Environment Variables

Create a `.env` file in `packages/server/` with the following keys:

```env
OPENAI_API_KEY=your_key_here
FIRECRAWL_API_KEY=your_key_here
TAVILY_API_KEY=your_key_here
CLERK_SECRET_KEY=your_key_here
EXPO_ACCESS_TOKEN=your_key_here
```

These will be automatically loaded by Docker Compose.

## Manual Commands

If you prefer to run commands manually:

```bash
# Build dev CLI
cargo build --release --bin dev

# Run dev CLI directly
./target/release/dev

# Start Expo manually
cd packages/app && npm start

# Docker commands manually
cd packages/server
docker compose up -d
docker compose restart
docker compose up -d --build
docker compose logs -f
```

## Project Structure

```
mndigitalaid/
├── dev.sh                 # Entry point script
├── packages/
│   ├── dev-cli/          # Rust CLI binary
│   ├── server/           # Rust API server
│   ├── app/              # Expo mobile app
│   ├── admin-spa/        # Admin web app
│   ├── seesaw-rs/        # Event-driven architecture
│   └── twilio-rs/        # Twilio integration
└── target/
    └── release/
        └── dev           # Built CLI binary
```

## Troubleshooting

### "Cargo is not installed"
Install Rust from [rustup.rs](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### "Docker is not installed"
Install Docker Desktop from [docker.com](https://www.docker.com/products/docker-desktop/)

### "Command not found: expo"
The CLI will install Expo CLI automatically on first run, or you can install it manually:
```bash
npm install -g expo-cli
```

### Port already in use
If you see port conflict errors:
- PostgreSQL (5432): Stop any local PostgreSQL instances
- Redis (6379): Stop any local Redis instances
- API (8080): Stop any other services using port 8080

### Docker services won't start
Check Docker Desktop is running and you have enough resources allocated (4GB RAM minimum recommended).

## Development Workflow

Typical workflow:
1. Run `./dev.sh`
2. Select "🐳 Docker start" to start backend services
3. Select "📱 Start mobile" to start the Expo app
4. Develop and test
5. Select "📋 Follow docker logs" to debug backend issues
6. Select "🔄 Docker restart" after making backend changes
