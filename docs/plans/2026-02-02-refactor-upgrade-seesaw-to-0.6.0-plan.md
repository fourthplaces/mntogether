---
title: "Upgrade to seesaw_core 0.6.0"
type: refactor
date: 2026-02-02
---

# Upgrade to seesaw_core 0.6.0

## Overview

Upgrade from seesaw_core 0.5.0 to 0.6.0. This is a **major architectural change** that introduces a Redux-style API with builder patterns for effects and reducers.

**Key Changes:**
- New `effect::on::<E>().run(handler)` builder pattern (replaces Effect trait impl)
- New `reducer::on::<E>().run(reducer)` builder pattern
- `Engine::with_deps(deps).with_effect().with_reducer()` builder (replaces EngineBuilder)
- `engine.activate(initial_state)` returns `Handle` (replaces run/dispatch_request)
- `handle.context.emit(event)` for emitting events
- `handle.settled().await` waits for all effects to complete
- New `Service` abstraction for action execution with `IntoAction` trait
- `EffectContext` now has `prev_state()`, `next_state()`, `curr_state()` methods
- No more `Effect` trait implementation - use closures

## API Changes Summary

| 0.5.0 | 0.6.0 |
|-------|-------|
| `impl Effect<E, D, S>` trait | `effect::on::<E>().run(\|event, ctx\| async {...})` |
| `Effect.handle(&mut self, event, ctx) -> Result<()>` | Closure: `\|Arc<E>, EffectContext<S, D>\| -> impl Future<Output = Result<()>>` |
| `EngineBuilder::new(deps).with_effect::<E, _>(effect)` | `Engine::with_deps(deps).with_effect(effect::on::<E>().run(...))` |
| `dispatch_request(event, &bus, matcher)` | `handle.context.emit(event)` + `handle.settled().await` |
| `ctx.deps()` | `ctx.deps()` (same) |
| `ctx.emit(event)` | `ctx.emit(event)` (same) |
| `RequestState` (universal state) | `S` (any state type, passed to `activate()`) |
| `Effect<E, D, S>` generic params | `Effect<S, D>` struct (event type via `on::<E>()`) |
| N/A | `effect::on::<E>().filter(\|e\| predicate).run(...)` |
| N/A | `effect::on_any().transition(\|prev, next\| changed).run(...)` |
| N/A | `effect::group([effect1, effect2, ...])` |
| N/A | `effect::task(\|ctx\| async {...})` for background tasks |
| N/A | `effect::bridge(tx.weak(), rx)` for relay connections |
| N/A | `Service::new(kernel).with_effect(...).run(state, action.with(opts))` |

## New Pattern

### Effect Registration (Builder Pattern)

**Old (0.5.0):**
```rust
pub struct AuthEffect;

#[async_trait]
impl Effect<AuthEvent, ServerDeps, RequestState> for AuthEffect {
    type Event = AuthEvent;

    async fn handle(
        &mut self,
        event: AuthEvent,
        ctx: EffectContext<ServerDeps, RequestState>,
    ) -> Result<()> {
        match event {
            AuthEvent::SendOTPRequested { phone_number } => {
                actions::handle_send_otp(phone_number, &ctx).await
            }
            AuthEvent::VerifyOTPRequested { phone_number, code } => {
                actions::handle_verify_otp(phone_number, code, &ctx).await
            }
            // Terminal events - no-op
            AuthEvent::OTPSent { .. } => Ok(()),
            AuthEvent::OTPVerified { .. } => Ok(()),
            _ => Ok(()),
        }
    }
}
```

**New (0.6.0):**
```rust
use seesaw_core::effect;

// Each event type gets its own effect handler
let auth_effects = effect::group([
    effect::on::<AuthEvent>()
        .filter(|e| matches!(e, AuthEvent::SendOTPRequested { .. }))
        .run(|event, ctx| async move {
            if let AuthEvent::SendOTPRequested { phone_number } = event.as_ref() {
                actions::handle_send_otp(phone_number.clone(), &ctx).await
            } else {
                Ok(())
            }
        }),
    effect::on::<AuthEvent>()
        .filter(|e| matches!(e, AuthEvent::VerifyOTPRequested { .. }))
        .run(|event, ctx| async move {
            if let AuthEvent::VerifyOTPRequested { phone_number, code } = event.as_ref() {
                actions::handle_verify_otp(phone_number.clone(), code.clone(), &ctx).await
            } else {
                Ok(())
            }
        }),
]);
```

**Or simpler with a single handler that matches:**
```rust
let auth_effect = effect::on::<AuthEvent>().run(|event, ctx| async move {
    match event.as_ref() {
        AuthEvent::SendOTPRequested { phone_number } => {
            actions::handle_send_otp(phone_number.clone(), &ctx).await
        }
        AuthEvent::VerifyOTPRequested { phone_number, code } => {
            actions::handle_verify_otp(phone_number.clone(), code.clone(), &ctx).await
        }
        // Terminal events - no-op
        _ => Ok(()),
    }
});
```

### Engine Setup

**Old (0.5.0):**
```rust
pub type AppEngine = Engine<ServerDeps, RequestState>;

let engine: AppEngine = EngineBuilder::new(server_deps)
    .with_effect::<AuthEvent, _>(AuthEffect)
    .with_effect::<MemberEvent, _>(MemberEffect)
    .with_effect::<ChatEvent, _>(ChatEffect)
    .build();

let bus = engine.bus().clone();
```

**New (0.6.0):**
```rust
use seesaw_core::{Engine, effect};

pub type AppEngine = Engine<AppState, ServerDeps>;

let engine: AppEngine = Engine::with_deps(server_deps)
    .with_effect(auth_effect)       // effect::on::<AuthEvent>().run(...)
    .with_effect(member_effect)     // effect::on::<MemberEvent>().run(...)
    .with_effect(chat_effect);      // effect::on::<ChatEvent>().run(...)

// No bus - use Service or handle.context.emit() instead
```

### GraphQL Integration

**Old (0.5.0):**
```rust
use seesaw_core::dispatch_request;

pub async fn send_verification_code(
    phone_number: String,
    ctx: &GraphQLContext,
) -> FieldResult<bool> {
    dispatch_request(
        AuthEvent::SendOTPRequested { phone_number },
        &ctx.bus,
        |m| {
            m.try_match(|e: &AuthEvent| match e {
                AuthEvent::OTPSent { .. } => Some(Ok(true)),
                AuthEvent::PhoneNotRegistered { .. } => Some(Err(FieldError::new(...))),
                _ => None,
            })
            .result()
        },
    )
    .await
    .map_err(...)
}
```

**New (0.6.0) - Option A: Direct Handle Usage:**
```rust
pub async fn send_verification_code(
    phone_number: String,
    ctx: &GraphQLContext,
) -> FieldResult<bool> {
    // Activate a new session with initial state
    let handle = ctx.engine.activate(AppState::default());

    // Emit the request event
    handle.context.emit(AuthEvent::SendOTPRequested {
        phone_number: phone_number.clone()
    });

    // Wait for effects to settle
    handle.settled().await
        .map_err(|e| FieldError::new(e.to_string(), Value::null()))?;

    // Check result from state (if using reducers) or return success
    Ok(true)
}
```

**New (0.6.0) - Option B: Service Pattern (Recommended):**
```rust
use seesaw_core::{Service, IntoAction, GenericActionResult, EffectContext};

pub async fn send_verification_code(
    phone_number: String,
    ctx: &GraphQLContext,
) -> FieldResult<bool> {
    ctx.service
        .run(
            AppState::default(),
            send_otp_action.with(phone_number),
        )
        .await
        .map_err(|e| FieldError::new(e.to_string(), Value::null()))
}

// Action function
async fn send_otp_action(
    phone_number: String,
    ctx: EffectContext<AppState, ServerDeps>,
) -> Result<GenericActionResult<bool>> {
    // Emit request event
    ctx.emit(AuthEvent::SendOTPRequested {
        phone_number: phone_number.clone()
    });

    // Action can do work directly
    let result = ctx.deps().twilio.send_otp(&phone_number).await?;

    // Emit fact event
    ctx.emit(AuthEvent::OTPSent { phone_number });

    Ok(GenericActionResult::new(result.is_some()))
}
```

### State Access

**Old (0.5.0):**
```rust
// Universal RequestState
ctx.state().visitor_id
```

**New (0.6.0):**
```rust
// prev_state() - state before reducer ran for this event
ctx.prev_state().some_field

// next_state() - state after reducer ran (snapshot at dispatch time)
ctx.next_state().some_field

// curr_state() - live state (reads current value, use in long-running tasks)
ctx.curr_state().some_field
```

### Reducers (New in 0.6.0)

```rust
use seesaw_core::reducer;

let counter_reducer = reducer::on::<Increment>().run(|state: AppState, event| {
    AppState {
        count: state.count + event.amount,
        ..state
    }
});

let engine = Engine::with_deps(deps)
    .with_reducer(counter_reducer)
    .with_effect(counter_effect);
```

## Migration Tasks

### Phase 1: Update Dependencies & Core Types

#### 1.1 Update Cargo.toml

```toml
# packages/server/Cargo.toml
seesaw_core = "0.6.0"
```

#### 1.2 Create Effect Builder Functions

Create a new file for each domain's effects using the builder pattern.

**File:** `domains/auth/effects.rs`

```rust
use seesaw_core::effect;
use super::events::AuthEvent;
use super::actions;
use crate::kernel::ServerDeps;
use crate::common::AppState;

/// Build the auth effect handler
pub fn auth_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<AuthEvent>().run(|event, ctx| async move {
        match event.as_ref() {
            AuthEvent::SendOTPRequested { phone_number } => {
                actions::handle_send_otp(phone_number.clone(), &ctx).await
            }
            AuthEvent::VerifyOTPRequested { phone_number, code } => {
                actions::handle_verify_otp(phone_number.clone(), code.clone(), &ctx).await
            }
            // Terminal events - no action needed
            AuthEvent::OTPSent { .. }
            | AuthEvent::OTPVerified { .. }
            | AuthEvent::OTPFailed { .. }
            | AuthEvent::PhoneNotRegistered { .. } => Ok(()),
        }
    })
}
```

#### 1.3 Create AppState

Replace `RequestState` with a domain-agnostic `AppState`:

**File:** `common/app_state.rs`

```rust
use uuid::Uuid;

/// Application state for the seesaw engine.
/// Updated by reducers when events are emitted.
#[derive(Clone, Default)]
pub struct AppState {
    pub visitor_id: Option<Uuid>,
    // Add domain-specific state fields as needed
}
```

#### 1.4 Update Action Signatures

Actions now receive `EffectContext<AppState, ServerDeps>` instead of `EffectContext<ServerDeps, RequestState>`.

**Pattern:**
```rust
// Before (0.5.0)
pub async fn handle_send_otp(
    phone_number: String,
    ctx: &EffectContext<ServerDeps, RequestState>,
) -> Result<()>

// After (0.6.0)
pub async fn handle_send_otp(
    phone_number: String,
    ctx: &EffectContext<AppState, ServerDeps>,
) -> Result<()>
```

### Phase 2: Update Engine Setup

#### 2.1 Update server/app.rs

```rust
use seesaw_core::Engine;
use crate::common::AppState;
use crate::domains::{
    auth::effects::auth_effect,
    member::effects::member_effect,
    chatrooms::effects::chat_effect,
    website::effects::website_effect,
    crawling::effects::crawler_effect,
    posts::effects::post_effect,
    domain_approval::effects::domain_approval_effect,
};

pub type AppEngine = Engine<AppState, ServerDeps>;

pub fn build_engine(deps: ServerDeps) -> AppEngine {
    Engine::with_deps(deps)
        .with_effect(auth_effect())
        .with_effect(member_effect())
        .with_effect(chat_effect())
        .with_effect(website_effect())
        .with_effect(crawler_effect())
        .with_effect(post_effect())
        .with_effect(domain_approval_effect())
}
```

#### 2.2 Update GraphQL Context

```rust
use seesaw_core::Service;
use crate::common::AppState;

pub type AppService = Service<AppState, ServerDeps>;

pub struct GraphQLContext {
    pub db_pool: PgPool,
    pub service: AppService,  // Replaces engine + bus
    pub auth_user: Option<AuthUser>,
    // ... other fields
}
```

### Phase 3: Convert Domain Effects

Convert each domain's effect implementation to the builder pattern.

#### 3.1 Auth Domain

**File:** `domains/auth/effects.rs`

```rust
use seesaw_core::effect;
use super::events::AuthEvent;
use super::actions;
use crate::kernel::ServerDeps;
use crate::common::AppState;

pub fn auth_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<AuthEvent>().run(|event, ctx| async move {
        match event.as_ref() {
            AuthEvent::SendOTPRequested { phone_number } => {
                actions::handle_send_otp(phone_number.clone(), &ctx).await
            }
            AuthEvent::VerifyOTPRequested { phone_number, code } => {
                actions::handle_verify_otp(phone_number.clone(), code.clone(), &ctx).await
            }
            _ => Ok(()),
        }
    })
}
```

#### 3.2 Member Domain

**File:** `domains/member/effects/mod.rs`

```rust
use seesaw_core::effect;
use super::events::MemberEvent;
use super::actions;
use crate::kernel::ServerDeps;
use crate::common::AppState;

pub fn member_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<MemberEvent>().run(|event, ctx| async move {
        match event.as_ref() {
            MemberEvent::RegisterMemberRequested { .. } => {
                actions::handle_register_member(event.as_ref().clone(), &ctx).await
            }
            MemberEvent::GenerateEmbeddingRequested { member_id } => {
                actions::handle_generate_embedding(*member_id, &ctx).await
            }
            MemberEvent::UpdateStatusRequested { member_id, status } => {
                actions::handle_update_status(*member_id, status.clone(), &ctx).await
            }
            _ => Ok(()),
        }
    })
}
```

#### 3.3 Chatrooms Domain

**File:** `domains/chatrooms/effects/chat.rs`

```rust
use seesaw_core::effect;
use super::super::events::ChatEvent;
use super::super::actions;
use crate::kernel::ServerDeps;
use crate::common::AppState;

pub fn chat_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<ChatEvent>().run(|event, ctx| async move {
        match event.as_ref() {
            ChatEvent::CreateContainerRequested { language, with_agent, requested_by } => {
                actions::handle_create_container(
                    language.clone(),
                    with_agent.clone(),
                    *requested_by,
                    &ctx,
                ).await
            }
            ChatEvent::SendMessageRequested { container_id, content, author_id } => {
                actions::handle_send_message(
                    *container_id,
                    content.clone(),
                    *author_id,
                    &ctx,
                ).await
            }
            ChatEvent::GenerateGreetingRequested { container_id, agent_config } => {
                actions::handle_generate_greeting(
                    *container_id,
                    agent_config.clone(),
                    &ctx,
                ).await
            }
            ChatEvent::GenerateReplyRequested { message_id, container_id } => {
                actions::handle_generate_reply(*message_id, *container_id, &ctx).await
            }
            _ => Ok(()),
        }
    })
}
```

#### 3.4 Crawling Domain

**File:** `domains/crawling/effects/crawler.rs`

```rust
use seesaw_core::effect;
use super::super::events::CrawlEvent;
use super::super::actions;
use crate::kernel::ServerDeps;
use crate::common::AppState;

pub fn crawler_effect() -> seesaw_core::effect::Effect<AppState, ServerDeps> {
    effect::on::<CrawlEvent>().run(|event, ctx| async move {
        match event.as_ref() {
            CrawlEvent::CrawlWebsiteRequested { website_id, job_id, requested_by, is_admin } => {
                actions::handle_crawl_website(
                    *website_id,
                    *job_id,
                    *requested_by,
                    *is_admin,
                    &ctx,
                ).await
            }
            CrawlEvent::RegeneratePostsRequested { page_snapshot_id, job_id, requested_by, is_admin } => {
                actions::handle_regenerate_posts(
                    *page_snapshot_id,
                    *job_id,
                    *requested_by,
                    *is_admin,
                    &ctx,
                ).await
            }
            _ => Ok(()),
        }
    })
}
```

### Phase 4: Update GraphQL Edges

Replace `dispatch_request` with Service pattern.

#### 4.1 Auth Edges

**File:** `domains/auth/edges/mutation.rs`

```rust
use seesaw_core::IntoAction;
use crate::common::AppState;
use crate::domains::auth::actions;

pub async fn send_verification_code(
    phone_number: String,
    ctx: &GraphQLContext,
) -> FieldResult<bool> {
    // Use service.run with action
    ctx.service
        .run(
            AppState::default(),
            actions::send_otp_action.with(phone_number),
        )
        .await
        .map_err(|e| FieldError::new(e.to_string(), Value::null()))
}

pub async fn verify_code(
    phone_number: String,
    code: String,
    ctx: &GraphQLContext,
) -> FieldResult<AuthResponse> {
    ctx.service
        .run(
            AppState::default(),
            actions::verify_otp_action.with((phone_number, code)),
        )
        .await
        .map_err(|e| FieldError::new(e.to_string(), Value::null()))
}
```

#### 4.2 Action Functions for GraphQL

**File:** `domains/auth/actions/send_otp.rs`

```rust
use anyhow::Result;
use seesaw_core::{GenericActionResult, EffectContext};
use crate::common::AppState;
use crate::kernel::ServerDeps;
use super::super::events::AuthEvent;

/// Action for sending OTP - callable from Service.run()
pub async fn send_otp_action(
    phone_number: String,
    ctx: EffectContext<AppState, ServerDeps>,
) -> Result<GenericActionResult<bool>> {
    // Emit request event (triggers effect handler)
    ctx.emit(AuthEvent::SendOTPRequested {
        phone_number: phone_number.clone()
    });

    // Or do work directly here
    let member = crate::domains::member::models::Member::find_by_phone(
        &phone_number,
        &ctx.deps().db_pool,
    ).await?;

    if member.is_none() {
        ctx.emit(AuthEvent::PhoneNotRegistered { phone_number });
        return Ok(GenericActionResult::new(false));
    }

    let identifier = ctx.deps().twilio.send_otp(&phone_number).await?;

    ctx.emit(AuthEvent::OTPSent {
        phone_number,
        identifier: identifier.unwrap_or_default(),
    });

    Ok(GenericActionResult::new(true))
}
```

### Phase 5: Update Tests

#### 5.1 Test Harness

**File:** `tests/common/harness.rs`

```rust
use seesaw_core::Engine;
use crate::common::AppState;

pub fn build_test_engine(deps: ServerDeps) -> Engine<AppState, ServerDeps> {
    crate::server::app::build_engine(deps)
}

// Test helper for running actions
pub async fn run_action<F, Fut, R>(
    engine: &Engine<AppState, ServerDeps>,
    action: F,
) -> Result<R>
where
    F: FnOnce(EffectContext<AppState, ServerDeps>) -> Fut,
    Fut: Future<Output = Result<R>>,
{
    let handle = engine.activate(AppState::default());
    let result = action(handle.context.clone()).await?;
    handle.settled().await?;
    Ok(result)
}
```

### Phase 6: Remove Unused Code

#### Files to DELETE:

| File | Reason |
|------|--------|
| `common/request_state.rs` | Replaced by `AppState` |
| Any `impl Effect<...>` blocks | Replaced by builder pattern |

#### Files to MODIFY:

| File | Change |
|------|--------|
| `common/mod.rs` | Export `AppState` instead of `RequestState` |
| `server/graphql/context.rs` | Use `Service` instead of `Engine + EventBus` |
| All `domains/*/effects/*.rs` | Convert to builder pattern |
| All `domains/*/edges/*.rs` | Use `Service.run()` pattern |

## File Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `common/app_state.rs` | Application state struct |

### Modified Files

| File | Change |
|------|--------|
| `Cargo.toml` | `seesaw_core = "0.6.0"` |
| `common/mod.rs` | Export `AppState` |
| `server/app.rs` | New engine builder pattern |
| `server/graphql/context.rs` | Use `Service` type |
| `domains/*/effects/*.rs` | Convert to `effect::on::<E>().run()` |
| `domains/*/edges/*.rs` | Use `Service.run()` with actions |
| `domains/*/actions/*.rs` | Update context type, add action functions |
| `tests/common/harness.rs` | Update test helpers |

### Deleted Files

| File | Reason |
|------|--------|
| `common/request_state.rs` | Replaced by `AppState` |

## Acceptance Criteria

- [x] `cargo build --package server` compiles with seesaw 0.6.2
- [x] All effects converted to `effect::on::<E>().run()` builder pattern
- [x] Engine uses `Engine::with_deps().with_effect()` pattern
- [x] GraphQL mutations use `engine.activate(state).process(|ctx| { ctx.emit(event); Ok(()) }).await` pattern
- [x] No `impl Effect<...>` trait implementations remain
- [x] No `dispatch_request()` calls remain
- [x] All tests compile (tests pass with test infrastructure)
- [x] `AppState` replaces `RequestState`

## Testing

```bash
cargo build --package server
cargo test --package server

# Manual testing via GraphQL playground
```

## Migration Order

1. **Update Cargo.toml** - get the new API
2. **Create AppState** - simple struct, minimal changes
3. **Convert auth effects** - simplest domain
4. **Update engine setup** - app.rs
5. **Convert remaining domains** - one at a time
6. **Update GraphQL edges** - use Service pattern
7. **Update tests** - fix harness
8. **Delete unused code** - RequestState, old imports

## Key Differences from 0.5.0

| Aspect | 0.5.0 | 0.6.0 |
|--------|-------|-------|
| Effect registration | Trait impl + EngineBuilder | Builder functions |
| Event handling | `match event {}` in trait method | Closure with `Arc<E>` |
| State | `RequestState` (universal) | Generic `S` (per-activation) |
| GraphQL integration | `dispatch_request()` + EventBus | `Service.run()` |
| Event emission | `ctx.emit(event)` | `ctx.emit(event)` (same) |
| Dependencies | `ctx.deps()` | `ctx.deps()` (same) |
| State access | `ctx.state()` | `ctx.prev_state()`, `ctx.next_state()`, `ctx.curr_state()` |
| Effect composition | Multiple `with_effect` calls | `effect::group([...])` |
| Background tasks | Within effects | `effect::task(...)` |
| Relay bridging | Manual | `effect::bridge(...)` |

## Benefits of 0.6.0

1. **Simpler effect registration** - No trait boilerplate
2. **Type-safe event filtering** - `.filter(|e| predicate)` at build time
3. **State transitions** - `.transition(|prev, next| changed)` for react-like effects
4. **Background tasks** - `effect::task(...)` for long-running work
5. **Relay integration** - `effect::bridge(...)` for bidirectional event streaming
6. **Service pattern** - Clean action execution with `IntoAction` trait
7. **Flexible state** - Pass any state type to `activate()`

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking API changes | High | Convert one domain at a time, test thoroughly |
| Closure captures | Medium | Use `Arc` for shared state, `clone()` for moves |
| State type changes | Medium | Audit all `ctx.state()` calls |
| Test failures | Low | Update test harness first |
| Event matching semantics change | High | Create spike branch to validate patterns first |
| Thread safety (Engine mutex removal) | High | Verify Engine is inherently thread-safe in 0.6.0 |

## Critical Open Questions

These questions need answers before starting implementation:

### Q1: How does Service.run() replace dispatch_request event matching? (CRITICAL)

**Current pattern:**
```rust
dispatch_request(
    AuthEvent::SendOTPRequested { phone_number },
    &ctx.bus,
    |m| m.try_match(|e: &AuthEvent| match e {
        AuthEvent::OTPSent { .. } => Some(Ok(true)),
        AuthEvent::PhoneNotRegistered { .. } => Some(Err(FieldError::new(...))),
        _ => None,
    }).result(),
)
```

**Problem:** Service.run() returns the action's result directly. How do we differentiate success/failure events?

**Proposed Answer:** In 0.6.0, the action function does the work directly and returns a result. Events are emitted for observability/side-effects but not for control flow. The action decides success/failure.

### Q2: Is the 0.6.0 Engine thread-safe without Arc<Mutex<...>>? (CRITICAL)

**Current:** `Arc<Mutex<AppEngine>>` in GraphQLContext
**0.6.0 Plan:** Direct engine usage with `engine.activate()` per request

**Proposed Answer:** Yes - Engine is designed for multiple concurrent `activate()` calls. Each Handle is isolated. The Engine itself is immutable after construction (effects/reducers are `Arc`-wrapped).

### Q3: What replaces ctx.state() in 0.6.0?

**Current:** `ctx.state().visitor_id`
**0.6.0:** Three options: `ctx.prev_state()`, `ctx.next_state()`, `ctx.curr_state()`

**Guidance:**
- `ctx.next_state()` - Use for most cases (state after this event's reducer ran)
- `ctx.curr_state()` - Use in long-running background tasks that need latest state
- `ctx.prev_state()` - Use for transition detection (comparing before/after)

### Q4: How to handle PostCompositeEffect's routing logic?

**Current:** 139 lines of routing between 4 sub-effects
**Proposed:** Single `effect::on::<PostEvent>().run(...)` with large match statement, or `effect::group([...])` with multiple filtered effects

**Recommendation:** Use single match statement (simpler, easier to follow), extract handler functions to actions module.

### Q5: Are reducers required in 0.6.0?

**Answer:** No. Reducers are optional. They update AppState synchronously before effects run. Use them when:
- You need state changes visible to effects immediately
- You want React-style "derived state" patterns
- You need state transitions to trigger effects via `.transition()`

For this codebase (action-based, not state-machine-based), reducers are likely NOT needed. Effects can use `ctx.deps()` for all data access.

## Domain-Specific Notes

### Posts Domain (Most Complex)

The posts domain has the most mutations (20+) and a composite effect. Migration order:

1. Extract PostCompositeEffect routing to single effect function
2. Convert all sub-effects (scraper, ai, sync, post) to action handlers
3. Update edges one at a time
4. Consider keeping PostEvent enum large but effects simple

### Crawling Domain (Cross-Domain Events)

Crawling emits `CrawlEvent::PagesReadyForExtraction` which triggers post extraction. In 0.6.0:

```rust
// posts/effects/extraction.rs
effect::on::<CrawlEvent>()
    .filter(|e| matches!(e, CrawlEvent::PagesReadyForExtraction { .. }))
    .run(|event, ctx| async move {
        // Cross-domain effect: posts listening to crawl events
        if let CrawlEvent::PagesReadyForExtraction { website_id, job_id, page_snapshot_ids } = event.as_ref() {
            actions::extract_posts_from_pages(*website_id, *job_id, page_snapshot_ids.clone(), &ctx).await
        } else {
            Ok(())
        }
    })
```

Register both effects on the engine:
```rust
Engine::with_deps(deps)
    .with_effect(crawler_effect())  // handles CrawlEvent::*Requested
    .with_effect(post_extraction_effect())  // handles CrawlEvent::PagesReadyForExtraction
```

### Job ID Correlation

Many async workflows use `job_id` for correlation:
```rust
let job_id = JobId::new();
ctx.emit(ScrapeSourceRequested { job_id, ... });
// Later, wait for PostsSynced { job_id }
```

In 0.6.0 with Service pattern, the action function owns the workflow:
```rust
async fn scrape_action(opts: ScrapeOpts, ctx: EffectContext<...>) -> Result<GenericActionResult<ScrapeResult>> {
    let job_id = JobId::new();

    // Do scraping work
    let pages = scrape_pages(opts.website_id, &ctx).await?;
    ctx.emit(PagesCrawled { job_id, pages: pages.clone() });

    // Do extraction
    let posts = extract_posts(pages, &ctx).await?;
    ctx.emit(PostsExtracted { job_id, posts: posts.clone() });

    // Sync
    let synced = sync_posts(posts, &ctx).await?;
    ctx.emit(PostsSynced { job_id, count: synced });

    Ok(GenericActionResult::new(ScrapeResult { job_id, synced }))
}
```

The action orchestrates the workflow, events are emitted for observability.
