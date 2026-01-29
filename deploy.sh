#!/bin/bash
# Deployment script for mndigitalaid infrastructure
# Usage: ./deploy.sh <env> <stack> [pulumi-args...]
#   env: dev or prod
#   stack: core, server, admin-spa, web-app, or all
#
# Examples:
#   ./deploy.sh dev core up --yes
#   ./deploy.sh prod all up --yes
#   ./deploy.sh dev server preview

set -e

if [ "$#" -lt 2 ]; then
  echo "Usage: $0 <env> <stack> [pulumi-args...]"
  echo ""
  echo "Arguments:"
  echo "  env: dev or prod"
  echo "  stack: core, server, web-app, web-next, or all"
  echo "  pulumi-args: additional Pulumi arguments (e.g., up --yes, preview, destroy)"
  echo ""
  echo "Examples:"
  echo "  $0 dev core up --yes"
  echo "  $0 prod all up --yes"
  echo "  $0 dev server preview"
  exit 1
fi

ENV=$1
STACK=$2
shift 2
PULUMI_ARGS="$@"

# Validate environment
if [ "$ENV" != "dev" ] && [ "$ENV" != "prod" ]; then
  echo "Error: env must be 'dev' or 'prod'"
  exit 1
fi

# Validate stack
VALID_STACKS=("core" "server" "web-app" "web-next" "all")
if [[ ! " ${VALID_STACKS[@]} " =~ " ${STACK} " ]]; then
  echo "Error: stack must be one of: ${VALID_STACKS[@]}"
  exit 1
fi

echo "========================================="
echo "Deploying mndigitalaid infrastructure"
echo "Environment: $ENV"
echo "Stack: $STACK"
echo "Pulumi args: $PULUMI_ARGS"
echo "========================================="
echo ""

cd infra

# Deploy a single stack
deploy_stack() {
  local stack_name=$1
  local stack_dir="packages/$stack_name"

  if [ ! -d "$stack_dir" ]; then
    echo "Error: Stack directory $stack_dir does not exist"
    exit 1
  fi

  echo ">>> Deploying $stack_name..."
  cd "$stack_dir"
  pulumi stack select "mntogether/$ENV" --create || pulumi stack select "mntogether/$ENV"
  pulumi $PULUMI_ARGS
  cd ../..
  echo ""
}

# Deploy stacks based on selection
if [ "$STACK" == "all" ]; then
  # Deploy in order: core -> server, web-app, web-next
  deploy_stack "core"
  deploy_stack "server"
  deploy_stack "web-app"
  deploy_stack "web-next"
else
  deploy_stack "$STACK"
fi

echo "========================================="
echo "Deployment complete!"
echo "========================================="
