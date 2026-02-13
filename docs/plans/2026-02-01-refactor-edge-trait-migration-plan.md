---
title: "refactor: Migrate Edges to Seesaw Edge Trait"
type: refactor
date: 2026-02-01
---

# refactor: Migrate Edges to Seesaw Edge Trait

## Overview

Refactor all GraphQL mutation edges to use the seesaw-core 0.4.0 `Edge` trait. Edges call actions directly and return fact events - no more `*Requested` event indirection.

## Problem Statement

The current pattern has unnecessary indirection:

```rust
// Current: Edge → dispatch_request → Effect → Event → Matcher
pub async fn create_chat(ctx: &GraphQLContext, ...) -> FieldResult<ContainerData> {
    let container_id = dispatch_request(
        ChatEvent::CreateContainerRequested { ... },  // Request event
        &ctx.bus,
        |m| { m.try_match(|e| ...).result() },       // Match response
    ).await?;

    let container = Container::find_by_id(container_id, &ctx.db_pool).await?;
    Ok(ContainerData::from(container))
}
```

Problems:
- Request/Response event pairs add complexity
- Matching logic scattered across mutations
- Effects just dispatch to actions anyway
- Unnecessary event bus round-trip

## Proposed Solution

Edges call actions directly and return fact events:

```rust
// New: Edge → Action → Fact Event (direct)
pub struct CreateChatEdge {
    pub language: String,
    pub with_agent: Option<String>,
    pub requested_by: Option<MemberId>,
}

impl Edge<ServerDeps> for CreateChatEdge {
    type Event = ChatEvent;
    type Data = Container;

    async fn execute(&self, ctx: &EdgeContext<ServerDeps>) -> Result<Self::Event> {
        // Call action directly - no *Requested event needed
        let container = actions::create_container(
            "ai_chat".to_string(),
            None,
            self.language.clone(),
            self.requested_by,
            self.with_agent.clone(),
            ctx.deps(),
        ).await?;

        Ok(ChatEvent::ContainerCreated {
            container_id: container.id,
            container_type: container.container_type.clone(),
            with_agent: self.with_agent.clone(),
        })
    }

    fn read(&self, state: &Self::Data) -> Option<Container> {
        Some(state.clone())
    }
}
```

## Technical Approach

### Architecture Change

```
OLD:  GraphQL → Edge func → dispatch_request(RequestEvent) → Effect → Action → FactEvent → Matcher
NEW:  GraphQL → Edge.execute() → Action → FactEvent (returned directly)
```

**Key insight:** Edges ARE the entry points. They can call actions directly and return the result. No need for request events or effects for simple mutations.

### When to Keep Effects

Effects are still needed for:
1. **Event chains** - When one fact event should trigger another workflow (internal edges)
2. **Background processing** - When work continues after the edge returns
3. **Cross-domain reactions** - When domain A's event triggers domain B's workflow

### Phase 1: Convert Simple Edges

Start with mutations that are simple request → action → response:

```rust
// domains/chatrooms/edges/create_chat.rs
use crate::domains::chatrooms::actions;

pub struct CreateChatEdge {
    pub language: String,
    pub with_agent: Option<String>,
    pub requested_by: Option<MemberId>,
}

impl Edge<ServerDeps> for CreateChatEdge {
    type Event = ChatEvent;
    type Data = ContainerData;

    async fn execute(&self, ctx: &EdgeContext<ServerDeps>) -> Result<Self::Event> {
        // 1. Auth check (if needed)
        // actions::check_auth(...)?;

        // 2. Call action directly
        let container = actions::create_container(
            "ai_chat".to_string(),
            None,
            self.language.clone(),
            self.requested_by,
            self.with_agent.clone(),
            ctx.deps(),
        ).await?;

        // 3. Return fact event
        Ok(ChatEvent::ContainerCreated {
            container_id: container.id,
            container_type: container.container_type.clone(),
            with_agent: self.with_agent.clone(),
        })
    }

    fn read(&self, result: &Container) -> Option<ContainerData> {
        Some(ContainerData::from(result.clone()))
    }
}
```

### Phase 2: Update GraphQL Resolvers

```rust
// server/graphql/mutation.rs
pub async fn create_chat(
    ctx: &GraphQLContext,
    language: Option<String>,
    with_agent: Option<String>,
) -> FieldResult<ContainerData> {
    let edge = CreateChatEdge {
        language: language.unwrap_or("en".to_string()),
        with_agent,
        requested_by: ctx.auth_user.as_ref().map(|u| u.member_id),
    };

    ctx.engine
        .run(edge)
        .await
        .map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))
}
```

### Phase 3: Remove Request Events

Once edges call actions directly, remove the `*Requested` event variants:

```rust
// BEFORE: events.rs
pub enum ChatEvent {
    // Request events (TO BE REMOVED)
    CreateContainerRequested { ... },
    SendMessageRequested { ... },

    // Fact events (KEEP)
    ContainerCreated { ... },
    MessageCreated { ... },
}

// AFTER: events.rs
pub enum ChatEvent {
    // Only fact events remain
    ContainerCreated { ... },
    MessageCreated { ... },
    MessageFailed { ... },
}
```

### Phase 4: Simplify Effects

Effects no longer handle request events. They only:
1. React to fact events for chains (internal edges)
2. Handle background processing

```rust
// BEFORE: Effect handles CreateContainerRequested
impl Effect<ChatEvent, ServerDeps> for ChatEffect {
    async fn handle(&mut self, event: ChatEvent, ctx: EffectContext<ServerDeps>) -> Result<Option<ChatEvent>> {
        match event {
            ChatEvent::CreateContainerRequested { ... } => {
                // This moves to Edge
            }
            _ => Ok(None),
        }
    }
}

// AFTER: Effect only handles chains/background work
impl Effect<ChatEvent, ServerDeps> for ChatEffect {
    async fn handle(&mut self, event: ChatEvent, ctx: EffectContext<ServerDeps>) -> Result<Option<ChatEvent>> {
        match event {
            // React to ContainerCreated to generate greeting (chain)
            ChatEvent::ContainerCreated { container_id, with_agent: Some(config), .. } => {
                let greeting = actions::generate_greeting(container_id, &config, ctx.deps()).await?;
                Ok(Some(ChatEvent::GreetingGenerated { ... }))
            }
            _ => Ok(None),
        }
    }
}
```

## Acceptance Criteria

- [ ] Edges call actions directly, no `dispatch_request`
- [ ] `*Requested` events removed from all domains
- [ ] Effects only handle event chains, not request events
- [ ] GraphQL resolvers use `engine.run(edge)`
- [ ] All existing functionality preserved

## Migration by Domain

### 1. Chatrooms Domain (Start Here)

| Mutation | Edge | Action to Call |
|----------|------|----------------|
| `create_chat` | `CreateChatEdge` | `actions::create_container()` |
| `send_message` | `SendMessageEdge` | `actions::create_message()` |
| `signal_typing` | Keep as direct emit | N/A (ephemeral) |

**Events to remove:** `CreateContainerRequested`, `SendMessageRequested`

### 2. Auth Domain

| Mutation | Edge | Action to Call |
|----------|------|----------------|
| `send_verification_code` | `SendOTPEdge` | `actions::send_otp()` |
| `verify_code` | `VerifyOTPEdge` | `actions::verify_otp()` |

**Events to remove:** `SendOTPRequested`, `VerifyOTPRequested`

### 3. Member Domain

| Mutation | Edge | Action to Call |
|----------|------|----------------|
| `register_member` | `RegisterMemberEdge` | `actions::register_member()` |
| `update_member_status` | `UpdateStatusEdge` | `actions::update_status()` |

**Events to remove:** `RegisterMemberRequested`, `UpdateMemberStatusRequested`

### 4. Website Domain

| Mutation | Edge | Action to Call |
|----------|------|----------------|
| `approve_website` | `ApproveWebsiteEdge` | `actions::approve_website()` |
| `reject_website` | `RejectWebsiteEdge` | `actions::reject_website()` |
| `suspend_website` | `SuspendWebsiteEdge` | `actions::suspend_website()` |

**Events to remove:** `ApproveWebsiteRequested`, `RejectWebsiteRequested`, `SuspendWebsiteRequested`

### 5. Posts Domain

| Mutation | Edge | Action to Call |
|----------|------|----------------|
| `submit_post` | `SubmitPostEdge` | `actions::submit_post()` |
| `approve_post` | `ApprovePostEdge` | `actions::approve_post()` |
| `reject_post` | `RejectPostEdge` | `actions::reject_post()` |
| ... | ... | ... |

**Events to remove:** All `*Requested` variants

### 6. Crawling Domain (Complex)

Crawling has multi-step workflows. The edge starts the workflow, effects handle chains:

```rust
pub struct CrawlWebsiteEdge { ... }

impl Edge<ServerDeps> for CrawlWebsiteEdge {
    async fn execute(&self, ctx: &EdgeContext<ServerDeps>) -> Result<Self::Event> {
        // Auth check
        actions::check_crawl_authorization(...)?;

        // Start crawl (first step only)
        let pages = actions::crawl_website_pages(...).await?;

        // Return fact event - effects handle the rest of the chain
        Ok(CrawlEvent::WebsiteCrawled {
            website_id: self.website_id,
            job_id: self.job_id,
            pages,
        })
    }
}

// Effect handles the chain: WebsiteCrawled → Extract → Sync
impl Effect<CrawlEvent, ServerDeps> for CrawlerEffect {
    async fn handle(&mut self, event: CrawlEvent, ctx: EffectContext<ServerDeps>) -> Result<Option<CrawlEvent>> {
        match event {
            CrawlEvent::WebsiteCrawled { pages, .. } => {
                let posts = actions::extract_posts_from_pages(&pages, ...).await?;
                Ok(Some(CrawlEvent::PostsExtracted { posts, .. }))
            }
            CrawlEvent::PostsExtracted { posts, .. } => {
                actions::sync_and_deduplicate_posts(posts, ...).await?;
                Ok(Some(CrawlEvent::PostsSynced { .. }))
            }
            _ => Ok(None),
        }
    }
}
```

## File Changes

### New Files

```
domains/{domain}/edges/
├── mod.rs              # Export edge structs
├── create_chat.rs      # CreateChatEdge
├── send_message.rs     # SendMessageEdge
└── ...
```

### Modified Files

- `domains/*/events/mod.rs` - Remove `*Requested` variants
- `domains/*/effects/*.rs` - Remove request event handlers
- `server/graphql/mutation.rs` - Use `engine.run(edge)`
- `server/app.rs` - Wire up edge execution

### Deleted Files

- `domains/*/edges/mutation.rs` - Replace with individual edge files

## Example: Full Chatrooms Migration

### Before

```rust
// edges/mutation.rs
pub async fn create_chat(ctx: &GraphQLContext, language: Option<String>, with_agent: Option<String>) -> FieldResult<ContainerData> {
    let container_id = dispatch_request(
        ChatEvent::CreateContainerRequested {
            container_type: "ai_chat".to_string(),
            entity_id: None,
            language: language.unwrap_or("en".to_string()),
            requested_by: ctx.auth_user.as_ref().map(|u| u.member_id),
            with_agent,
        },
        &ctx.bus,
        |m| m.try_match(|e: &ChatEvent| match e {
            ChatEvent::ContainerCreated { container_id, .. } => Some(Ok(*container_id)),
            _ => None,
        }).result(),
    ).await?;

    let container = Container::find_by_id(container_id, &ctx.db_pool).await?;
    Ok(ContainerData::from(container))
}

// effects/chat.rs
impl Effect<ChatEvent, ServerDeps> for ChatEffect {
    async fn handle(&mut self, event: ChatEvent, ctx: EffectContext<ServerDeps>) -> Result<Option<ChatEvent>> {
        match event {
            ChatEvent::CreateContainerRequested { container_type, entity_id, language, requested_by, with_agent } => {
                let container = actions::create_container(container_type, entity_id, language, requested_by, with_agent, &ctx).await?;
                Ok(Some(ChatEvent::ContainerCreated { container_id: container.id, ... }))
            }
            _ => Ok(None),
        }
    }
}
```

### After

```rust
// edges/create_chat.rs
pub struct CreateChatEdge {
    pub language: String,
    pub with_agent: Option<String>,
    pub requested_by: Option<MemberId>,
}

impl Edge<ServerDeps> for CreateChatEdge {
    type Event = ChatEvent;
    type Data = ContainerData;

    async fn execute(&self, ctx: &EdgeContext<ServerDeps>) -> Result<Self::Event> {
        let container = actions::create_container(
            "ai_chat".to_string(),
            None,
            self.language.clone(),
            self.requested_by,
            self.with_agent.clone(),
            ctx.deps(),
        ).await?;

        Ok(ChatEvent::ContainerCreated {
            container_id: container.id,
            container_type: container.container_type.clone(),
            with_agent: self.with_agent.clone(),
        })
    }

    fn read(&self, container: &Container) -> Option<ContainerData> {
        Some(ContainerData::from(container.clone()))
    }
}

// graphql/mutation.rs
pub async fn create_chat(ctx: &GraphQLContext, language: Option<String>, with_agent: Option<String>) -> FieldResult<ContainerData> {
    let edge = CreateChatEdge {
        language: language.unwrap_or("en".to_string()),
        with_agent,
        requested_by: ctx.auth_user.as_ref().map(|u| u.member_id),
    };

    ctx.engine.run(edge).await.map_err(|e| FieldError::new(e.to_string(), juniper::Value::null()))
}

// events/mod.rs - CreateContainerRequested REMOVED
pub enum ChatEvent {
    ContainerCreated { ... },  // Fact only
    MessageCreated { ... },
    // No more *Requested events
}
```

## Summary

| Aspect | Before | After |
|--------|--------|-------|
| Entry point | `dispatch_request()` function | `Edge` trait struct |
| Event flow | Request → Effect → Fact → Matcher | Edge → Action → Fact (direct) |
| Business logic | In Effect handlers | In Edge `execute()` via actions |
| Event types | Request + Fact pairs | Fact events only |
| Complexity | High (matching, bus round-trip) | Low (direct call) |

## References

- [seesaw-core 0.4.0 Edge trait](target/doc/seesaw_core/trait.Edge.html)
- [CLAUDE.md Seesaw Architecture Rules](/CLAUDE.md#seesaw-architecture-rules-v040)
- [Existing actions](packages/server/src/domains/chatrooms/actions/)
