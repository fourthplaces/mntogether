---
title: "Upgrade to seesaw_core 0.5.0"
type: refactor
date: 2026-02-01
---

# Upgrade to seesaw_core 0.5.0

## Overview

Upgrade from seesaw_core 0.4.0 to 0.5.0. The new version is much simpler:

- Effects return `Result<()>` and emit events via `ctx.emit()`
- No Edge/EdgeContext trait (removed from seesaw)
- No EventTap/TapContext trait (removed from seesaw)
- No domain-specific state or reducers - use Response.read() pattern
- Actions are plain functions returning Response structs with lazy `read()`

## API Changes Summary

| 0.4.0 | 0.5.0 |
|-------|-------|
| `Effect.handle() -> Result<Option<Event>>` | `Effect.handle() -> Result<()>` + `ctx.emit()` |
| `Edge` trait | Plain action functions |
| `EdgeContext` | `RunContext` |
| `EventTap` trait | Removed |
| `ChatRequestState` + `ChatReducer` | Universal `RequestState { visitor_id }` |
| `edge.read(state)` | `response.read().await` (queries DB) |

## New Pattern

### Key Principle: Actions emit facts, Effects handle chains

**Actions** do the work and emit fact events only:
- `ContainerCreated`, `MessageCreated`, etc.
- Never emit `*Requested` events (that's chaining logic)

**Effects** listen to facts and trigger chains:
- Listen for `ContainerCreated`, emit `GenerateGreetingRequested` if agent specified
- Listen for `MessageCreated`, emit `GenerateReplyRequested` if needed
- Handle `*Requested` events by calling actions and emitting results

This keeps actions clean and testable. Chain logic lives in one place (effects).

### Universal RequestState

One state struct for the entire app - just tracks visitor:

```rust
// common/request_state.rs
#[derive(Clone, Default)]
pub struct RequestState {
    pub visitor_id: Option<Uuid>,
}
```

### Response Pattern (replaces Edge.read + Reducer)

Response structs hold the ID and deps, `read()` queries DB directly:

```rust
// domains/chatrooms/actions/responses.rs
pub struct SendMessageResponse {
    pub id: Uuid,
    deps: ServerDeps,
}

impl SendMessageResponse {
    pub fn new(id: Uuid, deps: ServerDeps) -> Self {
        Self { id, deps }
    }

    pub async fn read(&self) -> Result<MessageData> {
        let message = Message::find_by_id(self.id, &self.deps.db_pool).await?;
        Ok(MessageData::from(message))
    }
}
```

### Action Functions (replace Edge structs)

Actions emit events and return Response:

```rust
// domains/chatrooms/actions/send_message.rs
pub async fn send_message(
    container_id: ContainerId,
    content: String,
    author_id: Option<MemberId>,
    ctx: &RunContext<'_, ServerDeps, RequestState>,
) -> Result<SendMessageResponse> {
    // Can access visitor_id from universal state
    let _visitor = ctx.state().visitor_id;

    // Do the work
    let message = Message::insert(
        container_id,
        "user".to_string(),
        content,
        author_id,
        None,
        &ctx.deps().db_pool,
    ).await?;

    // Emit fact event
    ctx.emit(ChatEvent::MessageCreated { message: message.clone() });

    // Return response with ID for lazy read
    Ok(SendMessageResponse::new(message.id, ctx.deps().clone()))
}
```

### Effects (handle chains, emit via ctx.emit())

Effects listen to fact events and trigger chains. Actions only emit facts.

```rust
#[async_trait]
impl Effect<ChatEvent, ServerDeps, RequestState> for ChatEffect {
    type Event = ChatEvent;

    async fn handle(
        &mut self,
        event: ChatEvent,
        ctx: EffectContext<ServerDeps, RequestState>,
    ) -> Result<()> {
        match event {
            // Fact event with chain: if agent specified, generate greeting
            ChatEvent::ContainerCreated { container, with_agent } => {
                if let Some(agent) = with_agent {
                    ctx.emit(ChatEvent::GenerateGreetingRequested {
                        container_id: container.id.into(),
                        agent_config: agent,
                    });
                }
                Ok(())
            }

            // Internal chain events
            ChatEvent::GenerateGreetingRequested { container_id, agent_config } => {
                let greeting = actions::generate_greeting(container_id, agent_config, ctx.deps()).await?;
                ctx.emit(ChatEvent::MessageCreated { message: greeting });
                Ok(())
            }

            ChatEvent::GenerateReplyRequested { message_id, container_id } => {
                let reply = actions::generate_reply(message_id, container_id, ctx.deps()).await?;
                ctx.emit(ChatEvent::MessageCreated { message: reply });
                Ok(())
            }

            // Terminal fact events - no further action
            ChatEvent::MessageCreated { .. } => Ok(()),
        }
    }
}
```

### GraphQL Usage

```rust
pub async fn send_message(
    ctx: &GraphQLContext,
    container_id: String,
    content: String,
) -> FieldResult<MessageData> {
    let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);
    let container_id = ContainerId::parse(&container_id)?;

    let mut engine = ctx.engine.lock().await;
    let response = engine
        .run(
            |run_ctx| actions::send_message(container_id, content, member_id, run_ctx),
            RequestState::default(),
        )
        .await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

    response.read().await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))
}
```

## Migration Tasks

### Phase 1: Fix Compilation Errors

#### 1.1 Create Universal RequestState

**File:** `common/request_state.rs` (new)

```rust
use uuid::Uuid;

/// Universal request state for all domains.
/// Passed through engine.run() for request-scoped data.
#[derive(Clone, Default)]
pub struct RequestState {
    pub visitor_id: Option<Uuid>,
}
```

**File:** `common/mod.rs` - add export

#### 1.2 Update Effect Return Types

Change from `Result<Option<Event>>` to `Result<()>`, emit via `ctx.emit()`.

**Files:**
- `domains/auth/effects.rs`
- `domains/chatrooms/effects/chat.rs`
- `domains/crawling/effects/crawler.rs`
- `domains/domain_approval/effects/mod.rs`
- `domains/member/effects/mod.rs`
- `domains/posts/effects/*.rs`
- `domains/website/effects/mod.rs`

**Pattern:**
```rust
// Before
async fn handle(...) -> Result<Option<ChatEvent>> {
    Ok(Some(ChatEvent::MessageCreated { message }))
}

// After
async fn handle(...) -> Result<()> {
    ctx.emit(ChatEvent::MessageCreated { message });
    Ok(())
}
```

#### 1.3 Remove EventTap Usage

**File:** `common/nats_tap.rs`

Option: Publish NATS directly in effects after `ctx.emit()`:

```rust
// In effect after emitting
ctx.emit(ChatEvent::MessageCreated { message: message.clone() });
if let Err(e) = publish_to_nats(&message, ctx.deps().nats.as_ref()).await {
    tracing::warn!("NATS publish failed: {}", e);
}
Ok(())
```

Or create a dedicated NATS effect that subscribes to events.

### Phase 2: Convert Chatrooms to Action Pattern

#### 2.1 Create Response Structs

**File:** `domains/chatrooms/actions/responses.rs` (new)

```rust
use anyhow::Result;
use uuid::Uuid;

use crate::common::ContainerId;
use crate::domains::chatrooms::data::{ContainerData, MessageData};
use crate::domains::chatrooms::models::{Container, Message};
use crate::kernel::ServerDeps;

pub struct CreateChatResponse {
    pub id: ContainerId,
    deps: ServerDeps,
}

impl CreateChatResponse {
    pub fn new(id: ContainerId, deps: ServerDeps) -> Self {
        Self { id, deps }
    }

    pub async fn read(&self) -> Result<ContainerData> {
        let container = Container::find_by_id(self.id, &self.deps.db_pool).await?;
        Ok(ContainerData::from(container))
    }
}

pub struct SendMessageResponse {
    pub id: Uuid,
    deps: ServerDeps,
}

impl SendMessageResponse {
    pub fn new(id: Uuid, deps: ServerDeps) -> Self {
        Self { id, deps }
    }

    pub async fn read(&self) -> Result<MessageData> {
        let message = Message::find_by_id(self.id, &self.deps.db_pool).await?;
        Ok(MessageData::from(message))
    }
}
```

#### 2.2 Create Action Functions

**File:** `domains/chatrooms/actions/create_chat.rs` (new)

```rust
use anyhow::Result;
use seesaw_core::RunContext;

use crate::common::{MemberId, RequestState};
use crate::domains::chatrooms::actions::responses::CreateChatResponse;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::Container;
use crate::kernel::ServerDeps;

pub async fn create_chat(
    language: String,
    with_agent: Option<String>,
    requested_by: Option<MemberId>,
    ctx: &RunContext<'_, ServerDeps, RequestState>,
) -> Result<CreateChatResponse> {
    let container = Container::insert(
        "ai_chat".to_string(),
        None,
        language,
        &ctx.deps().db_pool,
    ).await?;

    // Action only emits fact event - Effect handles chaining
    ctx.emit(ChatEvent::ContainerCreated {
        container: container.clone(),
        with_agent,
    });

    Ok(CreateChatResponse::new(container.id.into(), ctx.deps().clone()))
}
```

**File:** `domains/chatrooms/actions/send_message.rs` (new)

```rust
use anyhow::Result;
use seesaw_core::RunContext;

use crate::common::{ContainerId, MemberId, RequestState};
use crate::domains::chatrooms::actions::responses::SendMessageResponse;
use crate::domains::chatrooms::events::ChatEvent;
use crate::domains::chatrooms::models::Message;
use crate::kernel::ServerDeps;

pub async fn send_message(
    container_id: ContainerId,
    content: String,
    author_id: Option<MemberId>,
    ctx: &RunContext<'_, ServerDeps, RequestState>,
) -> Result<SendMessageResponse> {
    let message = Message::insert(
        container_id,
        "user".to_string(),
        content,
        author_id,
        None,
        &ctx.deps().db_pool,
    ).await?;

    ctx.emit(ChatEvent::MessageCreated { message: message.clone() });

    Ok(SendMessageResponse::new(message.id, ctx.deps().clone()))
}
```

#### 2.3 Update actions/mod.rs

```rust
mod create_chat;
mod create_container;
mod create_message;
mod generate_greeting;
mod generate_reply;
pub mod responses;
mod send_message;

pub use create_chat::create_chat;
pub use create_container::create_container;
pub use create_message::create_message;
pub use generate_greeting::generate_greeting;
pub use generate_reply::generate_reply;
pub use responses::*;
pub use send_message::send_message;
```

#### 2.4 Update GraphQL Mutations

**File:** `domains/chatrooms/edges/query.rs`

```rust
use crate::common::RequestState;
use crate::domains::chatrooms::actions;

pub async fn create_chat(
    ctx: &GraphQLContext,
    language: Option<String>,
    with_agent: Option<String>,
) -> FieldResult<ContainerData> {
    let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);
    let language = language.unwrap_or_else(|| "en".to_string());

    let mut engine = ctx.engine.lock().await;
    let response = engine
        .run(
            |run_ctx| actions::create_chat(language, with_agent, member_id, run_ctx),
            RequestState::default(),
        )
        .await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

    response.read().await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))
}

pub async fn send_message(
    ctx: &GraphQLContext,
    container_id: String,
    content: String,
) -> FieldResult<MessageData> {
    let member_id = ctx.auth_user.as_ref().map(|u| u.member_id);
    let container_id = ContainerId::parse(&container_id)?;

    let mut engine = ctx.engine.lock().await;
    let response = engine
        .run(
            |run_ctx| actions::send_message(container_id, content, member_id, run_ctx),
            RequestState::default(),
        )
        .await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))?;

    response.read().await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))
}
```

### Phase 3: Update Engine Setup

**File:** `server/graphql/context.rs`

```rust
use crate::common::RequestState;

pub type AppEngine = Engine<ServerDeps, RequestState>;
```

**File:** `server/app.rs`

```rust
use crate::common::RequestState;

// Remove .with_reducer() calls - no reducers needed
let engine: AppEngine = EngineBuilder::new(server_deps)
    .with_effect::<AuthEvent, _>(AuthEffect)
    .with_effect::<MemberEvent, _>(MemberEffect)
    .with_effect::<ChatEvent, _>(ChatEffect)
    .with_effect::<WebsiteEvent, _>(WebsiteEffect)
    .with_effect::<CrawlEvent, _>(CrawlerEffect)
    .with_effect::<PostEvent, _>(PostCompositeEffect::new())
    .with_effect::<DomainApprovalEvent, _>(DomainApprovalEffect)
    .build();
```

### Phase 4: Delete Unused Code

#### Files to DELETE:

| File | Reason |
|------|--------|
| `domains/chatrooms/edges/create_chat.rs` | Replaced by action |
| `domains/chatrooms/edges/send_message.rs` | Replaced by action |
| `domains/chatrooms/state.rs` | Replaced by universal RequestState |
| `domains/chatrooms/reducer.rs` | No longer needed - use Response.read() |

#### Update chatrooms/mod.rs:

Remove exports for deleted modules:
- Remove `pub mod state;`
- Remove `pub use reducer::ChatReducer;`

### Phase 5: Update Remaining Domains

Other domains using `dispatch_request` can stay as-is for now (it still works in 0.5.0).

Later, convert to action pattern domain by domain:
- auth
- member
- website
- crawling
- posts
- domain_approval

## File Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `common/request_state.rs` | Universal RequestState |
| `domains/chatrooms/actions/responses.rs` | Response structs |
| `domains/chatrooms/actions/create_chat.rs` | Action function |
| `domains/chatrooms/actions/send_message.rs` | Action function |

### Modified Files

| File | Change |
|------|--------|
| `Cargo.toml` | `seesaw_core = "0.5.0"` |
| `common/mod.rs` | Export RequestState |
| `domains/*/effects/*.rs` | Return `Result<()>`, use `ctx.emit()` |
| `domains/chatrooms/edges/query.rs` | Use action pattern |
| `domains/chatrooms/actions/mod.rs` | Export new actions |
| `domains/chatrooms/mod.rs` | Remove state/reducer exports |
| `server/app.rs` | Remove `.with_reducer()`, update engine type |
| `server/graphql/context.rs` | Update AppEngine type |
| `common/nats_tap.rs` | Remove EventTap, publish in effects |

### Deleted Files

| File | Reason |
|------|--------|
| `domains/chatrooms/edges/create_chat.rs` | Replaced by action |
| `domains/chatrooms/edges/send_message.rs` | Replaced by action |
| `domains/chatrooms/state.rs` | Replaced by RequestState |
| `domains/chatrooms/reducer.rs` | No reducers needed |

## Acceptance Criteria

- [ ] `cargo build --package server` compiles
- [ ] All effects return `Result<()>` and emit via `ctx.emit()`
- [ ] Universal `RequestState { visitor_id }` used everywhere
- [ ] No domain-specific state structs (ChatRequestState deleted)
- [ ] No reducers (ChatReducer deleted)
- [ ] Chatrooms uses action + Response.read() pattern
- [ ] GraphQL mutations work
- [ ] No `Edge`, `EdgeContext`, `EventTap`, `TapContext` imports
- [ ] All tests pass

## Testing

```bash
cargo build --package server
cargo test --package server

# Manual: create chat, send message via GraphQL
```
