---
title: Migrate from Seesaw to Restate.dev for Durable Execution
type: refactor
date: 2026-02-06
status: planning
complexity: high
estimated_duration: 3-4 months
risk_level: high
---

# Migrate from Seesaw to Restate.dev for Durable Execution

## Executive Summary

Migrate from the homegrown Seesaw event-driven framework (v0.10.3) to Restate.dev, a production-ready durable execution platform, while preserving the existing domain structure and business logic. This migration addresses documented pain points with Seesaw (3+ refactoring plans, reliability gaps, framework churn) and provides battle-tested durability, automatic retries, and better observability.

**Key Decision**: Choose Restate over Temporal because:
- ✅ Restate has an active, published Rust SDK (v0.4.0 on crates.io)
- ❌ Temporal's Rust SDK is pre-alpha and not production-ready
- ✅ Restate is lightweight (single binary, no external dependencies)
- ✅ Restate was built by Apache Flink creators and Meta infrastructure engineers

**Migration Approach**: Strangler fig pattern with feature flags, enabling incremental rollout and safe rollback.

---

## Problem Statement

### Current Pain Points (Documented)

From institutional research (`docs/plans/2026-02-*`):

**1. Seesaw Framework Immaturity**
- Homegrown framework requiring frequent upgrades (0.3.0 → 0.5.0 → 0.6.0 → 0.7.2 → 0.8.0 in 2 months)
- Not battle-tested at scale
- 3+ architectural refactoring plans addressing violations

**2. Reliability Gaps**
- **Job Pipeline Issues**: Monolithic execution means failure at step 2 forces re-execution of step 1
- **No Progress Visibility**: Only job-level status, no sub-step granularity
- **Cascading Failures**: One timeout re-runs all previous steps
- **Blocking Operations**: Admin actions (website crawl, approval) block user interactions for 30s-5min

**3. Architectural Violations**
- **20+ PLATINUM RULE violations**: Events like `ScrapeFailed`, `ExtractFailed` emitted on errors instead of using `Result::Err`
- **Fat Effects**: `posts/effects/crawler.rs` (1,620 lines), `crawling/effects/crawler.rs` (1,416 lines)
- **Cross-domain coupling**: No clear integration boundaries
- **Missing cascades**: Events defined but never emitted (`PagesReadyForExtraction`)

**4. Durability Issues**
- In-memory machine state (lost on restart)
- No event sourcing (events live in memory during execution)
- Custom lease-based job system without distributed transaction guarantees

### Why Restate Solves These Problems

| Problem | Seesaw | Restate |
|---------|--------|---------|
| **Durability** | In-memory state, lost on crash | Journal-based persistence, auto-resume |
| **Reliability** | Manual retry logic, custom circuit breakers | Built-in exponential backoff, infinite retries |
| **Observability** | Custom logging, no execution history | Built-in workflow UI, full execution trace |
| **Timeouts** | Single timeout for all steps | Per-step timeouts with automatic recovery |
| **Error Handling** | Manual PLATINUM RULE policing | Native exception boundaries, no "failed events" |
| **Framework Churn** | Frequent breaking changes | Stable v1.2 platform |
| **Blocking Ops** | Effects block user interactions | Async workflows don't block GraphQL |

---

## Proposed Solution

### Architecture Overview

**Current (Seesaw):**
```
GraphQL Mutation
  → queue_engine.process(event)
  → Event triggers Effect Handler
  → Effect emits new Event
  → Chain continues via Edges
  → State spread across events
```

**Target (Restate):**
```
GraphQL Mutation
  → workflow_client.start_workflow()
  → Workflow orchestrates steps
  → Each step is durable activity
  → State in workflow variables
  → Auto-retry, auto-resume
```

### Domain Structure Preservation

**Before:**
```
domains/crawling/
├── events/mod.rs         # Event enum
├── effects/pipeline.rs   # Effect handlers
├── actions/              # Business logic
│   ├── ingest_website.rs
│   └── post_extraction.rs
└── models/               # Data access
    └── extraction_page.rs
```

**After:**
```
domains/crawling/
├── workflows/            # NEW: Orchestration logic
│   ├── mod.rs
│   └── crawl.rs
├── activities/           # RENAMED: actions → activities
│   ├── ingest_website.rs  # UNCHANGED IMPLEMENTATION
│   └── post_extraction.rs # UNCHANGED IMPLEMENTATION
└── models/               # UNCHANGED
    └── extraction_page.rs
```

**Key Principle**: Business logic stays in activities (renamed from actions). Only orchestration moves to workflows.

---

## Technical Approach

### Phase 1: Foundation (Weeks 1-2)

**Setup Restate Infrastructure:**

1. **Install Restate binary:**
   ```bash
   # Download latest release
   curl -L https://github.com/restatedev/restate/releases/latest/download/restate-server-x86_64-unknown-linux-musl.tar.gz | tar xz

   # Or use Docker for development
   docker run -d --name restate \
     -p 8080:8080 -p 9070:9070 \
     -v restate-data:/restate-data \
     restatedev/restate:latest
   ```

2. **Add Rust SDK dependency:**
   ```toml
   # packages/server/Cargo.toml
   [dependencies]
   restate-sdk = "0.4.0"
   restate-sdk-testcontainers = "0.4.0"  # For testing
   ```

3. **Create Restate service bootstrap:**
   ```rust
   // packages/server/src/workflows/mod.rs
   use restate_sdk::prelude::*;

   pub async fn start_workflow_server() -> anyhow::Result<()> {
       HttpServer::new(
           Endpoint::builder()
               .with_service(crate::domains::crawling::workflows::CrawlWorkflowImpl.serve())
               .build(),
       )
       .listen_and_serve("0.0.0.0:9080".parse()?)
       .await
   }
   ```

**Deploy to Railway (Parallel to Existing Server):**

```yaml
# railway.yml
services:
  - name: main-server
    build:
      dockerfile: Dockerfile
    env:
      PORT: 8000

  - name: restate-runtime  # NEW
    image: restatedev/restate:1.2
    env:
      RESTATE_INGRESS_PORT: 9070
      RESTATE_ADMIN_PORT: 8080
    volumes:
      - /data:/restate-data

  - name: workflow-server  # NEW
    build:
      dockerfile: Dockerfile.workflows
    env:
      PORT: 9080
      RESTATE_RUNTIME_URL: http://restate-runtime:9070
```

### Phase 2: POC Workflow (Week 3)

**Migrate simplest workflow first**: `regenerate_posts_for_page`

**Step 1: Create workflow definition:**

```rust
// domains/crawling/workflows/regenerate.rs
use restate_sdk::prelude::*;
use uuid::Uuid;

#[restate_sdk::workflow]
pub trait RegeneratePostsWorkflow {
    async fn run(page_snapshot_id: Uuid) -> Result<usize, HandlerError>;
}

pub struct RegeneratePostsWorkflowImpl;

#[restate_sdk::workflow]
impl RegeneratePostsWorkflow for RegeneratePostsWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext,
        page_snapshot_id: Uuid,
    ) -> Result<usize, HandlerError> {
        // Step 1: Fetch context (durable, auto-retry)
        let page_ctx = ctx.run("fetch_context", || {
            // Call existing action! No changes to implementation
            activities::fetch_single_page_context(page_snapshot_id, &ctx.deps().db_pool)
        }).await?;

        // Early return if no context
        let Some(ctx_data) = page_ctx else {
            return Ok(0);
        };

        // Step 2: Extract posts (durable, auto-retry)
        let posts = ctx.run("extract_posts", || {
            activities::extract_posts_from_pages(
                &ctx_data.website,
                vec![build_page_from_snapshot(&ctx_data.page_snapshot)],
                ctx.deps(),
            )
        }).await?;

        if posts.is_empty() {
            return Ok(0);
        }

        // Step 3: Sync to database (durable, auto-retry)
        let count = posts.len();
        ctx.run("sync_posts", || {
            activities::sync_and_deduplicate_posts(
                ctx_data.website_id,
                posts,
                ctx.deps(),
            )
        }).await?;

        Ok(count)
    }
}
```

**Step 2: Add routing in GraphQL:**

```rust
// server/graphql/schema.rs
async fn regenerate_posts(
    ctx: &GraphQLContext,
    page_snapshot_id: Uuid,
) -> FieldResult<RegenerateResult> {
    // Feature flag for gradual rollout
    let use_workflows = ctx.feature_flags.workflow_enabled("regenerate_posts");

    if use_workflows {
        // NEW: Start Restate workflow
        let count = ctx.workflow_client
            .start_workflow(
                "regenerate_posts",
                page_snapshot_id,
            )
            .await
            .map_err(to_field_error)?;

        Ok(RegenerateResult { posts_regenerated: count })
    } else {
        // LEGACY: Use Seesaw
        let event = crawling_actions::regenerate_posts_for_page(
            page_snapshot_id,
            ctx.deps(),
        ).await.map_err(to_field_error)?;

        ctx.queue_engine.process(event).await.map_err(to_field_error)?;

        Ok(RegenerateResult { posts_regenerated: event.count })
    }
}
```

**Step 3: Shadow mode testing:**

```rust
// Run both systems in parallel, log differences
if ctx.feature_flags.workflow_shadow_mode("regenerate_posts") {
    tokio::spawn(async move {
        let workflow_result = workflow_client
            .start_workflow("regenerate_posts", page_snapshot_id)
            .await;

        let legacy_result = run_legacy_regenerate(page_snapshot_id).await;

        // Compare and alert if different
        metrics::compare_results("regenerate_posts", &legacy_result, &workflow_result);
    });
}
```

### Phase 3: Core Crawling Pipeline (Weeks 4-8)

**Map event chain to workflow:**

**Current Seesaw Pipeline:**
```
CrawlCommand::CrawlWebsite
  ↓ (effect handler)
CrawlEvent::PagesFetched { pages }
  ↓ (edge)
CrawlCommand::ExtractFromPages { pages }
  ↓ (effect handler)
CrawlEvent::PostsExtracted { posts }
  ↓ (edge)
CrawlCommand::SyncPosts { posts }
  ↓ (effect handler)
CrawlEvent::PostsSynced
```

**Restate Workflow:**

```rust
// domains/crawling/workflows/crawl.rs
use restate_sdk::prelude::*;

#[restate_sdk::workflow]
pub trait CrawlWebsiteWorkflow {
    async fn run(website_id: Uuid, visitor_id: Uuid, use_firecrawl: bool)
        -> Result<CrawlResult, HandlerError>;
}

pub struct CrawlWebsiteWorkflowImpl;

#[restate_sdk::workflow]
impl CrawlWebsiteWorkflow for CrawlWebsiteWorkflowImpl {
    async fn run(
        &self,
        ctx: WorkflowContext,
        website_id: Uuid,
        visitor_id: Uuid,
        use_firecrawl: bool,
    ) -> Result<CrawlResult, HandlerError> {
        // Authorization at GraphQL layer (before workflow starts)
        // Workflows assume authorization already passed

        // Step 1: Ingest website (10 min timeout)
        ctx.set_step_timeout(Duration::from_secs(600));
        let ingest_result = ctx.run("ingest_website", || {
            // Existing action! No changes needed
            activities::ingest_website(
                website_id,
                visitor_id,
                use_firecrawl,
                true,  // is_admin = true (already authorized)
                ctx.deps(),
            )
        }).await?;

        // Step 2: Extract narratives (2 min timeout)
        ctx.set_step_timeout(Duration::from_secs(120));
        let website = Website::find_by_id(website_id, &ctx.deps().db_pool).await?;
        let extraction = ctx.deps().extraction.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Extraction service not available"))?;

        let (narratives, _) = ctx.run("extract_narratives", || {
            activities::extract_narratives_for_domain(
                &website.domain,
                extraction.as_ref(),
            )
        }).await?;

        if narratives.is_empty() {
            return Ok(CrawlResult {
                website_id,
                posts_synced: 0,
                status: "no_narratives_found".to_string(),
            });
        }

        // Step 3: Investigate posts in parallel (fan-out)
        ctx.set_step_timeout(Duration::from_secs(120));
        let posts: Vec<ExtractedPost> = ctx.parallel("investigate_posts",
            narratives.into_iter().map(|narrative| {
                ctx.run(format!("investigate_{}", narrative.title), || async {
                    activities::investigate_post(&narrative, ctx.deps()).await
                })
            })
        ).await?;

        // Step 4: Sync and deduplicate
        ctx.set_step_timeout(Duration::from_secs(300));
        let synced_count = ctx.run("sync_posts", || {
            activities::llm_sync_posts(website_id, posts, ctx.deps())
        }).await?;

        Ok(CrawlResult {
            website_id,
            posts_synced: synced_count,
            status: "completed".to_string(),
        })
    }
}
```

**GraphQL Integration:**

```rust
async fn crawl_website(
    ctx: &GraphQLContext,
    website_id: Uuid,
) -> FieldResult<ScrapeJobResult> {
    let user = ctx.auth_user.as_ref()
        .ok_or_else(|| FieldError::new("Authentication required", Value::null()))?;

    // Authorization check (before workflow)
    Actor::new(user.member_id, user.is_admin)
        .can(AdminCapability::TriggerScraping)
        .check(ctx.deps())
        .await
        .map_err(to_field_error)?;

    let use_workflows = ctx.feature_flags.workflow_enabled("crawl_website");

    if use_workflows {
        // Start workflow (non-blocking)
        let workflow_id = ctx.workflow_client
            .start_workflow(
                "crawl_website",
                CrawlRequest {
                    website_id,
                    visitor_id: user.member_id.into_uuid(),
                    use_firecrawl: true,
                },
            )
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: workflow_id,
            status: "enqueued".to_string(),
            message: Some("Crawl started in background".to_string()),
        })
    } else {
        // Legacy path
        let handle = ctx.queue_engine
            .process(CrawlEvent::CrawlWebsiteEnqueued {
                website_id,
                visitor_id: user.member_id.into_uuid(),
                use_firecrawl: true,
            })
            .await
            .map_err(to_field_error)?;

        Ok(ScrapeJobResult {
            job_id: handle.correlation_id,
            status: "enqueued".to_string(),
            message: Some("Crawl enqueued for background processing".to_string()),
        })
    }
}
```

### Phase 4: Remaining Domains (Weeks 9-12)

**Migration Priority:**

1. ✅ Week 9: `crawling/regenerate_single_post` (done in Phase 2)
2. ✅ Week 10: `crawling/crawl_website` (done in Phase 3)
3. Week 11: `posts/submit_resource_link` (scrape → extract → create pipeline)
4. Week 12: `discovery/discover_website` (Tavily search → ingest → extract)

**For each domain:**
- Create `domains/{domain}/workflows/` directory
- Move orchestration logic from `effects/` to `workflows/`
- Rename `actions/` → `activities/` (no code changes!)
- Add feature flag routing in GraphQL
- Shadow mode testing for 1 week
- Gradual rollout: 1% → 10% → 50% → 100%

### Phase 5: Deprecation (Month 4)

**Week 13-14: Cooling Period**
- All workflows on Restate (100% traffic)
- Keep Seesaw running but idle
- Monitor for any regressions

**Week 15-16: Cleanup**
- Remove Seesaw dependency from `Cargo.toml`
- Delete `domains/*/effects/` directories
- Archive event definitions (keep for audit trail)
- Update `CLAUDE.md` with new architecture rules

---

## Migration Checklist

### Pre-Migration

- [ ] Set up Restate runtime locally (Docker)
- [ ] Add Restate Rust SDK to dependencies
- [ ] Create feature flag system for gradual rollout
- [ ] Set up monitoring/alerting for workflows
- [ ] Document rollback procedure

### Per-Domain Migration

- [ ] Identify event chains in domain
- [ ] Create `workflows/` directory
- [ ] Convert effect handlers → workflow handlers
- [ ] Rename `actions/` → `activities/` (preserve git history)
- [ ] Add feature flag routing in GraphQL
- [ ] Write integration tests
- [ ] Shadow mode testing (1 week)
- [ ] Gradual rollout: 1% → 10% → 50% → 100%
- [ ] Monitor metrics (completion rate, duration, errors)
- [ ] Delete legacy effects after 30 days

### Post-Migration

- [ ] Remove Seesaw from dependencies
- [ ] Archive event definitions
- [ ] Update CLAUDE.md architecture rules
- [ ] Document new workflow patterns
- [ ] Team training on Restate debugging

---

## Success Metrics

**Reliability:**
- ✅ 99.9%+ workflow completion rate (vs 95% with Seesaw)
- ✅ Zero lost jobs due to crashes (vs manual re-triggering)
- ✅ <1% error rate increase during migration

**Performance:**
- ✅ Workflow latency within 2x of legacy system
- ✅ No user-facing blocking operations (async workflows)
- ✅ p95 end-to-end crawl time <10 minutes

**Developer Experience:**
- ✅ Single-file workflows (vs scattered effects)
- ✅ Built-in observability (vs custom logging)
- ✅ Zero framework upgrade churn (stable v1.2)

---

## Risk Analysis & Mitigation

### Risk 1: Restate Maturity (HIGH)

**Risk**: Restate v1.2 is relatively new (Nov 2024 GA), limited production track record.

**Mitigation**:
- Start with non-critical workflows (regenerate posts)
- Keep Seesaw running in parallel for 90 days
- Feature flags enable instant rollback
- Comprehensive monitoring and alerting

### Risk 2: Rust SDK Stability (MEDIUM)

**Risk**: Rust SDK is less mature than TypeScript/Java SDKs.

**Mitigation**:
- Test extensively in dev/staging
- Shadow mode testing with real traffic
- Contribute fixes upstream if issues found
- Maintain close communication with Restate team

### Risk 3: State Migration (MEDIUM)

**Risk**: In-flight jobs during migration could be lost.

**Mitigation**:
- Dual-write pattern during transition
- Snapshot Seesaw state before cutover
- Manual replay procedure for lost jobs
- Drain Seesaw queue before final cutover

### Risk 4: Team Learning Curve (LOW)

**Risk**: Team needs to learn durable execution patterns.

**Mitigation**:
- Workshop on durable execution (1 day)
- Pair programming during first migration
- Document common patterns in CLAUDE.md
- Code reviews emphasize best practices

### Risk 5: Deployment Complexity (LOW)

**Risk**: Additional infrastructure (Restate runtime + workflow server).

**Mitigation**:
- Restate is single binary (simple deployment)
- Railway supports multi-service deployments
- Docker Compose for local development
- Rollback is just toggling feature flags

---

## Rollback Plan

### Instant Rollback (Feature Flags)

```rust
// Toggle environment variable
USE_WORKFLOWS=false

// Or database flag
UPDATE feature_flags SET enabled = false WHERE name = 'workflows_enabled';
```

**Result**: All traffic instantly routes to Seesaw.

### State Reconciliation After Rollback

```sql
-- Find workflows in progress
SELECT * FROM crawl_jobs
WHERE execution_engine = 'workflow'
AND status IN ('in_progress', 'pending');

-- Re-emit to Seesaw
-- (Manual procedure documented in runbook)
```

### Cooling Period

- Keep Restate running for 30 days after rollback
- Analyze failure root cause
- Fix issues before re-attempting migration

---

## Testing Strategy

### Unit Tests (Per Workflow)

```rust
#[tokio::test]
async fn test_crawl_workflow() {
    let test_env = RestateSdkTest::new().await.unwrap();

    let result = test_env
        .workflow_client()
        .start_workflow("crawl_website", CrawlRequest {
            website_id: test_uuid(),
            visitor_id: test_uuid(),
            use_firecrawl: false,
        })
        .await
        .unwrap();

    assert_eq!(result.status, "completed");
    assert!(result.posts_synced > 0);
}
```

### Integration Tests (Shadow Mode)

```rust
// Run legacy + workflow in parallel
// Compare outcomes
#[tokio::test]
async fn test_crawl_equivalence() {
    let website_id = seed_test_website().await;

    let (legacy_result, workflow_result) = tokio::join!(
        run_legacy_crawl(website_id),
        run_workflow_crawl(website_id),
    );

    assert_eq!(legacy_result.posts_count, workflow_result.posts_count);
}
```

### Chaos Tests (Resilience)

```rust
#[tokio::test]
async fn test_workflow_crash_recovery() {
    let test_env = RestateSdkTest::new().await.unwrap();

    let workflow_id = test_env.start_workflow("crawl", req).await?;

    // Simulate crash after step 2
    test_env.kill_worker_after_step(2).await;
    test_env.restart_worker().await;

    // Verify completion without re-executing steps 1-2
    let result = test_env.await_workflow(workflow_id).await?;
    assert_eq!(result.status, "completed");
}
```

---

## Dependencies

### New Dependencies

```toml
[dependencies]
restate-sdk = "0.4.0"

[dev-dependencies]
restate-sdk-testcontainers = "0.4.0"
```

### Infrastructure

- **Restate Runtime**: Docker image or binary
- **Persistent Storage**: Existing PostgreSQL (for state snapshots)
- **Railway Services**: +2 services (restate-runtime, workflow-server)

### No Breaking Changes

- ✅ Existing models unchanged
- ✅ GraphQL schema unchanged (internal routing only)
- ✅ Database schema unchanged
- ✅ Business logic unchanged (activities = actions)

---

## Alternative Approaches Considered

### Option 1: Temporal

**Pros**: Most mature durable execution platform, extensive production use
**Cons**: Rust SDK is pre-alpha, not production-ready in 2026
**Verdict**: ❌ Not viable for Rust-first project

### Option 2: PostgreSQL Job Queue

**Pros**: Simple, proven, no new dependencies
**Cons**: Manual retry logic, no workflow UI, no automatic resume
**Verdict**: ⚠️ Viable fallback if Restate doesn't work out

### Option 3: Keep Seesaw, Fix Issues

**Pros**: No migration cost, team familiar with it
**Cons**: Homegrown framework, ongoing maintenance burden, no external support
**Verdict**: ❌ Technical debt will compound over time

### Option 4: Inngest

**Pros**: Modern, growing, simple API
**Cons**: Rust SDK not as mature as Restate, less control over infrastructure
**Verdict**: ⚠️ Worth revisiting if Restate doesn't meet needs

---

## Future Considerations

### Workflow Versioning

```rust
#[restate_sdk::workflow(version = "v2")]
impl CrawlWorkflow for CrawlWorkflowV2Impl {
    // New implementation
    // Old workflows continue on v1
}
```

### Cross-Domain Orchestration

```rust
// Parent workflow calls child workflows
async fn content_pipeline(&self, ctx: WorkflowContext) -> Result<()> {
    let crawl_result = ctx.call_workflow("crawl_website", req).await?;
    ctx.call_workflow("moderate_posts", crawl_result.posts).await?;
    Ok(())
}
```

### Workflow Analytics

```rust
// Built-in observability
// Restate UI shows:
// - Active workflows
// - Execution history
// - Step-by-step timeline
// - Retry attempts
// - Error traces
```

---

## Documentation Plan

### Update CLAUDE.md

**New section: Restate Workflow Architecture**

```markdown
## Restate Workflow Rules

### HARD RULE: Workflows Orchestrate, Activities Contain Logic

Workflows define multi-step processes. Activities contain business logic.

✅ Correct:
```rust
async fn run(&self, ctx: WorkflowContext) -> Result<()> {
    let pages = ctx.run("fetch", || fetch_pages()).await?;
    let posts = ctx.run("extract", || extract_posts(pages)).await?;
    Ok(())
}
```

❌ Incorrect:
```rust
async fn run(&self, ctx: WorkflowContext) -> Result<()> {
    // 100 lines of parsing logic
    let posts = parse_html(&html);
    // Don't put business logic in workflows!
}
```

### Workflows Replace Effects

- OLD: `domains/*/effects/` with event handlers
- NEW: `domains/*/workflows/` with workflow handlers
- Actions renamed to activities (same implementation!)

### GraphQL Integration

Mutations start workflows via `workflow_client.start_workflow()`:

```rust
async fn crawl_website(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<JobType> {
    // Auth check
    check_authorization(ctx)?;

    // Start workflow
    let job_id = ctx.workflow_client
        .start_workflow("crawl_website", req)
        .await?;

    Ok(JobType { id: job_id })
}
```

No business logic in GraphQL layer!
```

### Team Runbook

**Create `docs/runbooks/restate-workflows.md`:**

- How to start a workflow
- How to debug failed workflows
- How to query workflow state
- How to manually retry
- How to rollback to Seesaw
- Common error patterns

---

## Implementation Timeline

### Month 1: Foundation
- Week 1-2: Setup + POC (regenerate posts workflow)
- Week 3-4: Shadow mode testing + metrics

### Month 2: Core Pipelines
- Week 5-6: Crawl website workflow
- Week 7-8: Post extraction workflow

### Month 3: Full Migration
- Week 9-10: Remaining workflows (discovery, submission)
- Week 11-12: 100% traffic cutover

### Month 4: Cleanup
- Week 13-14: Cooling period (monitor for issues)
- Week 15-16: Deprecate Seesaw, update docs

**Total Duration**: 3-4 months

---

## Acceptance Criteria

### Functional Requirements

- [ ] All Seesaw workflows migrated to Restate
- [ ] GraphQL API unchanged (internal routing only)
- [ ] No regressions in crawl success rate
- [ ] Automatic retry on transient failures
- [ ] Workflow state persisted across restarts

### Non-Functional Requirements

- [ ] 99.9%+ workflow completion rate
- [ ] <2x latency increase vs legacy
- [ ] Zero lost jobs during migration
- [ ] <1 hour to rollback if needed
- [ ] Built-in workflow observability

### Quality Gates

- [ ] 100% test coverage for workflows
- [ ] Shadow mode testing: 1000+ executions with <1% divergence
- [ ] Chaos testing: Workflows recover from crashes
- [ ] Load testing: Handle 10x current traffic
- [ ] Team training: All engineers can debug workflows

---

## References

### Internal Documentation

- Current architecture: `/packages/server/src/domains/crawling/effects/pipeline.rs:1-200`
- Seesaw pain points: `docs/plans/2026-02-03-refactor-event-effect-chain-architecture-plan.md`
- Job pipeline issues: `docs/plans/2026-02-04-refactor-split-crawl-pipeline-into-jobs-plan.md`
- Seesaw upgrade plans: `docs/plans/2026-02-05-refactor-upgrade-seesaw-to-0.8.0-plan.md`

### External Resources

- [Restate Documentation](https://docs.restate.dev/)
- [Restate Rust SDK](https://docs.rs/restate-sdk/latest/restate_sdk/)
- [Restate Examples](https://github.com/restatedev/examples)
- [Durable Execution Concepts](https://docs.restate.dev/concepts/durable_execution/)
- [Why We Built Restate](https://www.restate.dev/blog/why-we-built-restate)
- [Strangler Fig Pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/strangler-fig)

---

## Open Questions

1. **Performance Benchmarking**: What's acceptable latency increase? (Target: <2x)
2. **Monitoring**: Which metrics dashboard tool? (Grafana, Datadog, native Restate UI?)
3. **Alerting**: What workflow failures trigger pages? (All? Critical only?)
4. **Rollout Pace**: How long at each traffic percentage? (1 week per milestone?)
5. **State Migration**: Replay in-flight Seesaw jobs or let them complete first?

---

## Notes

- This is a large refactoring affecting core business logic
- Incremental migration reduces risk significantly
- Feature flags enable instant rollback if issues arise
- Business logic (activities) unchanged - only orchestration moves
- Restate's maturity is main risk factor (mitigated by gradual rollout)
