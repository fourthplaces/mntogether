#!/bin/bash
# Register workflow server with Restate runtime
#
# This script registers the workflow server endpoint with Restate
# so that Restate can proxy workflow invocations to it.

set -e

RESTATE_ADMIN=${RESTATE_ADMIN_URL:-http://localhost:9070}
RESTATE_INGRESS=${RESTATE_INGRESS:-http://localhost:8180}
WORKFLOW_SERVER=${WORKFLOW_SERVER_URL:-http://server:9080}

echo "Registering workflow server with Restate..."
echo "  Restate Admin API: $RESTATE_ADMIN"
echo "  Workflow Server:   $WORKFLOW_SERVER"

# Register the workflow server endpoint
curl -X POST "$RESTATE_ADMIN/deployments" \
  -H "Content-Type: application/json" \
  -d "{\"uri\": \"$WORKFLOW_SERVER\", \"force\": true}"

echo ""
echo "Workflow server registered successfully!"
echo ""
echo "Available workflows:"
echo "  - SendOtp"
echo "  - VerifyOtp"
echo "  - CrawlWebsite"
echo ""
echo "Invoke workflows via: $RESTATE_INGRESS/<WorkflowName>/run"
echo ""

# Bootstrap scheduled task loops
echo "Bootstrapping scheduled tasks..."

# Discovery search loop (hourly)
curl -s -X POST "$RESTATE_INGRESS/Discovery/run_scheduled_discovery/send" \
  -H "idempotency-key: scheduled-discovery-loop" \
  -H "Content-Type: application/json" \
  -d '{}' || true

# Website scraping loop (hourly)
curl -s -X POST "$RESTATE_INGRESS/Websites/run_scheduled_scrape/send" \
  -H "idempotency-key: scheduled-scrape-loop" \
  -H "Content-Type: application/json" \
  -d '{}' || true

# Weekly notification reset
curl -s -X POST "$RESTATE_INGRESS/Members/run_weekly_reset/send" \
  -H "idempotency-key: scheduled-weekly-reset" \
  -H "Content-Type: application/json" \
  -d '{}' || true

echo ""
echo "Scheduled tasks bootstrapped!"
echo "  - Discovery search (hourly loop)"
echo "  - Website scraping (hourly loop)"
echo "  - Weekly notification reset"
