#!/bin/bash
# Quick start script for development
#
# This script starts all required services for development:
# 1. Infrastructure (Postgres, Redis, NATS, Restate)
# 2. Provides instructions for starting the server

set -e

echo "Starting Development Environment"
echo ""

# Start infrastructure
echo "Starting infrastructure services..."
docker-compose up -d postgres redis nats restate

# Wait for services to be healthy
echo "Waiting for services to be healthy..."
sleep 5

# Check health
echo "Checking service health..."
docker-compose ps postgres redis nats restate

echo ""
echo "Infrastructure ready!"
echo ""
echo "Next steps:"
echo ""
echo "1. Start server (Terminal 1):"
echo "    cd packages/server"
echo "    cargo run --bin server"
echo ""
echo "2. Register services with Restate (after server starts):"
echo "    ./scripts/register-workflows.sh"
echo ""
echo "3. Test:"
echo "    See TESTING_WORKFLOWS.md for test commands"
echo ""
echo "Service URLs:"
echo "  Restate Ingress:   http://localhost:8180"
echo "  Restate Admin:     http://localhost:9070"
echo "  Server:            http://localhost:9080"
echo "  SSE:               http://localhost:8081"
echo ""
