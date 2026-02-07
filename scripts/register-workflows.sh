#!/bin/bash
# Register workflow server with Restate runtime
#
# This script registers the workflow server endpoint with Restate
# so that Restate can proxy workflow invocations to it.

set -e

RESTATE_ADMIN=${RESTATE_ADMIN_URL:-http://localhost:9071}
WORKFLOW_SERVER=${WORKFLOW_SERVER_URL:-http://workflow-server:9080}

echo "Registering workflow server with Restate..."
echo "  Restate Admin API: $RESTATE_ADMIN"
echo "  Workflow Server:   $WORKFLOW_SERVER"

# Register the workflow server endpoint
curl -X POST "$RESTATE_ADMIN/deployments" \
  -H "Content-Type: application/json" \
  -d "{\"uri\": \"$WORKFLOW_SERVER\"}"

echo ""
echo "✅ Workflow server registered successfully!"
echo ""
echo "Available workflows:"
echo "  - SendOtp"
echo "  - VerifyOtp"
echo "  - CrawlWebsite"
echo ""
echo "Invoke workflows via: http://localhost:9070/<WorkflowName>/run"
