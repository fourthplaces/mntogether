#!/bin/bash
# Quick start script for Restate workflow development
#
# This script starts all required services for workflow development:
# 1. Infrastructure (Postgres, Redis, NATS, Restate)
# 2. Registers workflow server with Restate
# 3. Provides instructions for starting dev servers

set -e

echo "🚀 Starting Restate Workflow Development Environment"
echo ""

# Start infrastructure
echo "📦 Starting infrastructure services..."
docker-compose up -d postgres redis nats restate

# Wait for services to be healthy
echo "⏳ Waiting for services to be healthy..."
sleep 5

# Check health
echo "🏥 Checking service health..."
docker-compose ps postgres redis nats restate

echo ""
echo "✅ Infrastructure ready!"
echo ""
echo "📝 Next steps:"
echo ""
echo "1️⃣  Start workflow server (Terminal 1):"
echo "    cd packages/server"
echo "    cargo run --bin workflow_server"
echo ""
echo "2️⃣  Register workflows with Restate (after workflow server starts):"
echo "    ./scripts/register-workflows.sh"
echo ""
echo "3️⃣  Start API server (Terminal 2):"
echo "    cd packages/server"
echo "    cargo run --bin server"
echo ""
echo "4️⃣  Test workflows:"
echo "    See TESTING_WORKFLOWS.md for test commands"
echo ""
echo "🔗 Service URLs:"
echo "  • GraphQL API:       http://localhost:8080/graphql"
echo "  • Restate Ingress:   http://localhost:9070"
echo "  • Restate Admin:     http://localhost:9071"
echo "  • Workflow Server:   http://localhost:9080"
echo ""
