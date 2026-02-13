# Restate Migration - Current Blocker

## Status: Blocked on Serialization Requirements

### What We've Discovered

After implementing the correct trait-based workflow pattern from official Restate SDK documentation, we've hit a fundamental issue:

**Restate SDK's `ctx.run()` requires ALL return types to be serializable for durable journaling.**

### The Core Problem

```rust
// This fails because AuthEvent doesn't implement restate_sdk::serde::Serialize
let event = ctx
    .run(|| async {
        activities::send_otp(phone_number, &deps)
            .await
            .map_err(Into::into)  // Returns Result<AuthEvent, HandlerError>
    })
    .await?;
```

**Error:**
```
error[E0277]: the trait bound `AuthEvent: restate_sdk::serde::Deserialize` is not satisfied
```

### Why This Happens

1. **Restate journals all results**: `ctx.run()` persists return values to enable deterministic replay
2. **Our domain events aren't compatible**: Types like `AuthEvent`, `CrawlEvent`, `Website`, etc. derive `serde::Serialize` but need `restate_sdk::serde::Serialize`
3. **Deep serialization requirements**: Even intermediate types (tuples, nested structs) must be serializable

### Affected Types

All of these would need to be made serializable with Restate's version of serde:

- `AuthEvent` (enum with multiple variants)
- `CrawlEvent` (enum with multiple variants)
- `Website` (complex model with many fields)
- `NarrativePost` (from extraction library)
- `ExtractedPost`, `ExtractedPostInformation`
- Many other domain types

### What We've Tried

1. ✅ Correct trait-based workflow pattern (works)
2. ✅ `WorkflowContext<'_>` instead of `Context<'_>` (works)
3. ✅ `Json<>` wrappers for request/response types (works)
4. ✅ Proper error conversion with `.map_err(Into::into)` (works)
5. ❌ Using `ctx.run()` for operations that return domain events (blocked)

### The Fundamental Issue

**Our architecture is incompatible with Restate's workflow pattern.**

Our current code:
- Activities return domain events (e.g., `AuthEvent::OTPSent`)
- Events are rich enums with many variants
- Events flow through Seesaw's effect handlers
- Workflows try to journal these events for durability

Restate expects:
- All journaled data to be simple, serializable types
- Workflows to return business data, not events
- Event-driven patterns to be replaced with workflow orchestration

### Two Paths Forward

#### Option A: Minimal Restate Integration (Recommended)

**Don't migrate everything to Restate workflows.** Instead:

1. Keep Seesaw for domain events (auth, crawling, etc.)
2. Use Restate ONLY for:
   - Long-running background jobs
   - Scheduled/cron tasks
   - Operations that need guaranteed execution
3. Workflows return simple data, not domain events
4. Fire-and-forget pattern from GraphQL → Seesaw events

**Benefits:**
- Minimal code changes
- Hybrid approach uses each tool appropriately
- Seesaw handles event-driven architecture
- Restate handles durable execution

**Example:**
```rust
// GraphQL mutation
async fn crawl_website(ctx: &GraphQLContext, website_id: Uuid) -> FieldResult<bool> {
    // Just emit event via Seesaw
    let event = CrawlEvent::CrawlRequested { website_id, ... };
    ctx.engine.emit(event);
    Ok(true)
}

// Restate workflow (separate, for retry logic)
#[restate_sdk::workflow]
pub trait RetryableCrawl {
    async fn execute(website_id: String) -> Result<Json<CrawlStats>, HandlerError>;
}
```

#### Option B: Full Restate Migration (Not Recommended)

Requires rewriting the entire domain architecture:

1. Remove all domain events (AuthEvent, CrawlEvent, etc.)
2. Make all activities return simple data (no events)
3. Add Serialize/Deserialize to 50+ types
4. Rewrite all effect handlers as workflow steps
5. Remove Seesaw completely
6. Change GraphQL mutations to call workflows directly

**Estimated effort:** 40+ hours of refactoring

### Recommendation

**Go with Option A: Hybrid Seesaw + Restate**

Use Restate for what it's good at (durable execution, retries, scheduling) and keep Seesaw for event-driven architecture.

### Next Steps

1. Revert workflow files to simpler stubs
2. Keep Seesaw for all domain event flows
3. Add Restate workflows ONLY for:
   - Scheduled crawls
   - Retry-heavy operations
   - Background jobs
4. Document hybrid architecture in CLAUDE.md

### References

- [Restate Rust SDK](https://github.com/restatedev/sdk-rust)
- [Restate Documentation](https://docs.restate.dev/category/rust-sdk/)
- [Workflow Example](https://docs.rs/restate-sdk/latest/restate_sdk/attr.workflow.html)

---

**Bottom Line:** The original "aggressive migration to Restate" approach is architecturally incompatible with our event-driven codebase. A hybrid approach is more pragmatic.
