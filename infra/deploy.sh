#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
ENV=${ENV:-dev}
PULUMI_ARGS=${PULUMI_ARGS:-up --yes}
STACK_NAME=${STACK_NAME:-}

# Help message
show_help() {
    cat << EOF
Usage: ./deploy.sh [OPTIONS]

Deploy infrastructure stacks to AWS using Pulumi

OPTIONS:
    -e, --env ENV           Environment to deploy (dev|prod) [default: dev]
    -s, --stack STACK       Specific stack to deploy (core|server|web-app|web|all)
    -h, --help              Show this help message

EXAMPLES:
    # Deploy all stacks to dev
    ./deploy.sh

    # Deploy all stacks to prod
    ./deploy.sh -e prod

    # Deploy only server stack to dev
    ./deploy.sh -s server

    # Deploy core and server to prod
    ./deploy.sh -e prod -s core
    ./deploy.sh -e prod -s server

    # Preview changes without deploying
    ENV=dev PULUMI_ARGS="preview" ./deploy.sh

    # Destroy a stack
    ENV=dev PULUMI_ARGS="destroy --yes" ./deploy.sh -s server

EOF
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--env)
            ENV="$2"
            shift 2
            ;;
        -s|--stack)
            STACK_NAME="$2"
            shift 2
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_help
            exit 1
            ;;
    esac
done

# Validate environment
if [[ "$ENV" != "dev" && "$ENV" != "prod" ]]; then
    echo -e "${RED}Invalid environment: $ENV. Must be 'dev' or 'prod'${NC}"
    exit 1
fi

echo -e "${GREEN}Deploying to environment: $ENV${NC}"
echo -e "${GREEN}Pulumi command: $PULUMI_ARGS${NC}"
echo ""

# Function to deploy a stack
deploy_stack() {
    local stack_name=$1
    local display_name=$2

    echo -e "${YELLOW}========================================${NC}"
    echo -e "${YELLOW}Deploying: $display_name${NC}"
    echo -e "${YELLOW}========================================${NC}"

    cd "packages/$stack_name"

    # Select or create stack with organization
    pulumi stack select "mntogether/$ENV" --create 2>/dev/null || pulumi stack select "mntogether/$ENV"

    # Run pulumi command
    pulumi $PULUMI_ARGS

    cd ../..

    echo -e "${GREEN}✓ Completed: $display_name${NC}"
    echo ""
}

# Deploy stacks in dependency order
if [[ -z "$STACK_NAME" || "$STACK_NAME" == "all" || "$STACK_NAME" == "core" ]]; then
    deploy_stack "core" "Core Infrastructure (VPC, RDS, Certificates)"
fi

if [[ -z "$STACK_NAME" || "$STACK_NAME" == "all" || "$STACK_NAME" == "server" ]]; then
    deploy_stack "server" "API Server (ECS Fargate)"
fi

if [[ -z "$STACK_NAME" || "$STACK_NAME" == "all" || "$STACK_NAME" == "web-app" ]]; then
    deploy_stack "web-app" "Web App (CloudFront + S3)"
fi

if [[ -z "$STACK_NAME" || "$STACK_NAME" == "all" || "$STACK_NAME" == "web" ]]; then
    deploy_stack "web" "Next.js App (ECS Fargate)"
fi

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Deployment Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
