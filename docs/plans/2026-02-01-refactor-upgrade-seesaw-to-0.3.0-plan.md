---
title: Upgrade Seesaw to 0.3.0
type: refactor
date: 2026-02-01
---

# Upgrade Seesaw to 0.3.0

## Overview

Upgrade seesaw-core from 0.1.1 to 0.3.0. This is a major architectural change that removes Machines and Commands in favor of a simpler **Event → Effect → Event** flow. The new architecture also introduces Edges as clean entry points and optional Reducers for state management.

## Problem Statement / Motivation

The current 0.1.1 architecture has an intermediate layer (Machines + Commands) that adds complexity without proportional benefit:

**Current (0.1.1):**
```
Edge → dispatch_request → Event → Machine.decide() → Command → Effect.execute() → Event
```

**Target (0.3.0):**
```
Edge → Event → Effect.handle() → Event → Effect.handle() → ... (until settled)
```

Benefits of 0.3.0:
- **Simpler mental model**: Events flow directly through Effects
- **Less boilerplate**: No Command enums, no Machine trait implementations
- **Cleaner Effects**: Effects receive events directly, not commands
- **Stateful Effects**: Effects now have `&mut self`, enabling in-effect state
- **Clean entry points**: Edges API provides structured workflow initiation

## Current State Analysis

### Domains Affected

| Domain | Machines | Commands | Effects | Stateful? | Background Jobs |
|--------|----------|----------|---------|-----------|-----------------|
| Auth | 1 (`AuthMachine`) | 2 variants | 1 | No | No |
| Member | 1 (`MemberMachine`) | 2 variants | 1 | Yes (HashMap) | Yes |
| Chat | 4 | 4 variants | 3 | No | Yes (TODO) |
| Domain Approval | 1 | 3 variants | 3 (composite) | Yes (HashMap) | No |
| Crawling | 1 | Multiple | 1 | Yes (HashSet) | No |
| Posts | 1 | Many | Multiple | Yes (HashSet) | No |
| Website | 1 | Multiple | 1 | No | No |

### Key Files

**Core Registration:**
- `packages/server/src/server/app.rs` - EngineBuilder setup (lines 163-195)

**Per-Domain Files:**
- `domains/{domain}/machines.rs` or `machines/mod.rs` - DELETE
- `domains/{domain}/commands.rs` or `commands/mod.rs` - DELETE
- `domains/{domain}/actions/` - CREATE (new directory for business logic)
- `domains/{domain}/jobs/` - CREATE (typed background jobs, if needed)
- `domains/{domain}/effects.rs` or `effects/mod.rs` - MODIFY (thin dispatcher to actions)
- `domains/{domain}/edges/*.rs` - MODIFY (can call actions directly)

## Proposed Solution

### Phase 1: Simple Domains (Auth, Website)

Start with domains that have:
- Single machine
- No stateful tracking
- No background jobs
- Straightforward event chains

### Phase 2: Moderate Complexity (Member, Domain Approval)

Domains with:
- Stateful tracking (HashMaps)
- Background jobs
- Multi-step workflows

### Phase 3: Complex Domains (Chat)

Four-machine orchestration that chains multiple effects.

### Phase 4: Cross-Domain (Crawling, Posts)

Domains with cross-domain event listening patterns.

## Technical Approach

### Architecture Change Summary

**Old Effect Signature (0.1.1):**
```rust
#[async_trait]
impl Effect<AuthCommand, ServerDeps> for AuthEffect {
    type Event = AuthEvent;

    async fn execute(
        &self,
        cmd: AuthCommand,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<AuthEvent> {
        match cmd {
            AuthCommand::SendOTP { phone_number } => { /* ... */ }
        }
    }
}
```

**New Effect Signature (0.3.0):**
```rust
#[async_trait]
impl Effect<AuthEvent, ServerDeps> for AuthEffect {
    type Event = AuthEvent;

    async fn handle(
        &mut self,
        event: AuthEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<AuthEvent> {
        match event {
            AuthEvent::SendOTPRequested { phone_number } => {
                // Do IO
                ctx.deps().twilio.send_otp(&phone_number).await?;
                Ok(AuthEvent::OTPSent { phone_number })
            }
            // Effects only handle events they're registered for
        }
    }
}
```

**Key Differences:**
| Aspect | 0.1.1 | 0.3.0 |
|--------|-------|-------|
| Input | Command | Event |
| Self | `&self` | `&mut self` |
| Method | `execute()` | `handle()` |
| Return | `Result<Event>` | `Result<Event>` |
| Routing | Via Machine | Direct registration |

### EngineBuilder Change

**Old (0.1.1):**
```rust
EngineBuilder::new(server_deps)
    .with_machine(AuthMachine)
    .with_effect::<AuthCommand, _>(AuthEffect)
    .build()
```

**New (0.3.0):**
```rust
EngineBuilder::new(server_deps)
    .with_effect::<AuthEvent, _>(AuthEffect)
    .build()
```

### Handling Multi-Step Workflows

In 0.3.0, effects are registered for specific event types. When an event is emitted, only the registered effect handles it:

```
AuthEvent::SendOTPRequested
    ↓
AuthEffect.handle() → Ok(AuthEvent::OTPSent)
    ↓
(Runtime emits OTPSent to bus)
    ↓
(No effect registered for OTPSent → flow settles)
```

**Key insight:** Effects only receive events they're registered for. No more `Ok(None)` for "not my event".

### Handling Stateful Machines

Current machines store state (e.g., `requesters: HashMap<Uuid, MemberId>`). Two options:

**Option A: Stateful Effects (Recommended for Simple State)**
```rust
struct DomainApprovalEffect {
    requesters: HashMap<Uuid, MemberId>,  // Effect is now &mut self
}

impl Effect<DomainApprovalEvent, ServerDeps> for DomainApprovalEffect {
    type Event = DomainApprovalEvent;

    async fn handle(&mut self, event: DomainApprovalEvent, ctx: EffectContext<ServerDeps>) -> Result<Self::Event> {
        match event {
            DomainApprovalEvent::AssessWebsiteRequested { domain_id, job_id, requested_by } => {
                self.requesters.insert(job_id, requested_by);  // Store state
                // ... do work, return next event
            }
            // Handle other events this effect is registered for
        }
    }
}
```

**Option B: Reducers (For State Shared Across Effects)**
```rust
struct DomainApprovalReducer;

impl Reducer<DomainApprovalEvent, RequestState> for DomainApprovalReducer {
    fn reduce(&self, state: &RequestState, event: &DomainApprovalEvent) -> RequestState {
        match event {
            DomainApprovalEvent::AssessWebsiteRequested { job_id, requested_by, .. } => {
                let mut new_state = state.clone();
                new_state.requesters.insert(job_id, requested_by);
                new_state
            }
            _ => state.clone(),
        }
    }
}
```

### Handling Background Jobs

Background jobs use a typed `Job` trait for type-safety and self-documenting code.

**Job Trait Definition:**
```rust
// In seesaw-job-postgres or local jobs module
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait Job: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static {
    type Event: Event;

    /// Unique job type identifier (e.g., "member.generate_embedding")
    fn job_type() -> &'static str;

    /// Execute the job, returning an event to emit
    async fn execute<D: Send + Sync>(
        &self,
        ctx: &EffectContext<D>,
    ) -> Result<Self::Event>;

    /// Optional: idempotency key for deduplication
    fn idempotency_key(&self) -> Option<String> {
        None
    }

    /// Optional: max retry attempts (default: 3)
    fn max_retries(&self) -> u32 {
        3
    }

    /// Optional: priority (higher = sooner, default: 0)
    fn priority(&self) -> i32 {
        0
    }
}
```

**Define Jobs in Domain:**
```rust
// domains/member/jobs/generate_embedding.rs
use seesaw_job_postgres::Job;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateEmbeddingJob {
    pub member_id: MemberId,
}

#[async_trait]
impl Job for GenerateEmbeddingJob {
    type Event = MemberEvent;

    fn job_type() -> &'static str {
        "member.generate_embedding"
    }

    fn idempotency_key(&self) -> Option<String> {
        Some(format!("member_embedding:{}", self.member_id))
    }

    async fn execute<D: Send + Sync>(
        &self,
        ctx: &EffectContext<D>,
    ) -> Result<MemberEvent> {
        let member = Member::find_by_id(self.member_id, &ctx.deps().db_pool).await?;
        let embedding = ctx.deps().ai.embed(&member.bio).await?;
        Member::update_embedding(self.member_id, embedding, &ctx.deps().db_pool).await?;

        Ok(MemberEvent::EmbeddingGenerated {
            member_id: self.member_id,
        })
    }
}
```

**Enqueue from Effect or Action:**
```rust
// In an action or effect
MemberEvent::MemberRegistered { member_id, .. } => {
    // Type-safe job enqueueing
    ctx.deps().job_queue.enqueue(GenerateEmbeddingJob { member_id }).await?;

    Ok(MemberEvent::EmbeddingScheduled { member_id })  // Acknowledge, job runs async
}
```

**Job Directory Structure:**
```
domains/{domain}/
├── actions/
├── effects.rs
├── events.rs
├── jobs/              # NEW - background job definitions
│   ├── mod.rs
│   └── generate_embedding.rs
└── mod.rs
```

**Benefits of Typed Jobs:**
| Aspect | String + JSON | Job Trait |
|--------|---------------|-----------|
| Type safety | Runtime errors | Compile-time |
| Params | `json!({ ... })` | Struct fields |
| Config | Separate | Self-contained |
| Execute | External handler | `job.execute()` |
| Testing | Manual setup | Direct invocation |

### Handling Cross-Domain Events

Effects can listen to events from any domain by registering for that event type:

```rust
// In posts domain, listening to crawling events
struct PostExtractionEffect;

impl Effect<CrawlEvent, ServerDeps> for PostExtractionEffect {
    type Event = PostExtractionEvent;  // Returns different type!

    async fn handle(&mut self, event: CrawlEvent, ctx: EffectContext<ServerDeps>) -> Result<PostExtractionEvent> {
        // Only registered for PagesReadyForExtraction variant
        let CrawlEvent::PagesReadyForExtraction { page_snapshot_ids, .. } = event;
        let posts = extract_posts(page_snapshot_ids, ctx.deps()).await?;
        Ok(PostExtractionEvent::PostsExtractedFromPages { posts })
    }
}
```

### Edge API (New in 0.3.0)

Edges provide clean entry points that emit initial events and read final results:

```rust
struct SendOTPEdge {
    phone_number: String,
}

impl Edge<RequestState> for SendOTPEdge {
    type Event = AuthEvent;
    type Data = bool;

    fn execute(&self, _ctx: &EdgeContext<RequestState>) -> Option<AuthEvent> {
        Some(AuthEvent::SendOTPRequested {
            phone_number: self.phone_number.clone(),
        })
    }

    fn read(&self, state: &RequestState) -> Option<Self::Data> {
        state.otp_sent.then_some(true)
    }
}

// Usage in GraphQL
pub async fn send_verification_code(phone_number: String, ctx: &GraphQLContext) -> FieldResult<bool> {
    ctx.engine
        .run(SendOTPEdge { phone_number }, RequestState::default())
        .await?
        .ok_or_else(|| FieldError::new("OTP send failed", Value::null()))
}
```

**Migration Decision:** Edges are optional. You can continue using `dispatch_request()` with the EventBus. Consider Edges for:
- Complex workflows with state tracking
- Clean separation of entry point logic
- Type-safe result extraction

### Actions Directory Pattern (NEW)

Actions are reusable business logic functions that can be called from both **Edges** and **Effects**. This pattern extracts the actual work into standalone functions, keeping Effects thin.

**Directory Structure:**
```
domains/{domain}/
├── actions/           # NEW - reusable business logic
│   ├── mod.rs
│   ├── send_otp.rs
│   └── verify_otp.rs
├── effects.rs         # Thin - dispatches to actions
├── edges/             # Can also call actions directly
│   └── mutation.rs
└── events.rs
```

**Action Signature:**
```rust
// domains/auth/actions/send_otp.rs
use crate::server::ServerDeps;
use seesaw_core::EffectContext;
use anyhow::Result;

pub async fn send_otp(
    phone_number: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<AuthEvent> {
    // All business logic lives here
    let identifier = ctx.deps().twilio.send_otp(&phone_number).await?;

    match identifier {
        Some(id) => Ok(AuthEvent::OTPSent {
            phone_number,
            identifier: id,
        }),
        None => Ok(AuthEvent::PhoneNotRegistered { phone_number }),
    }
}
```

**Effect Using Action (Thin Dispatcher):**
```rust
// domains/auth/effects.rs
use super::actions;

impl Effect<AuthEvent, ServerDeps> for AuthEffect {
    type Event = AuthEvent;

    async fn handle(
        &mut self,
        event: AuthEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<AuthEvent> {
        match event {
            AuthEvent::SendOTPRequested { phone_number } => {
                actions::send_otp(phone_number, &ctx).await
            }
            AuthEvent::VerifyOTPRequested { identifier, code, phone_number } => {
                actions::verify_otp(identifier, code, phone_number, &ctx).await
            }
        }
    }
}
```

**Edge Using Action Directly:**
```rust
// domains/auth/edges/mutation.rs
use super::super::actions;

impl Edge<RequestState> for SendOTPEdge {
    type Data = AuthEvent;

    fn execute(&self, _ctx: &EdgeContext<RequestState>) -> Option<Box<dyn Event>> {
        // Edge emits request event to trigger flow
        Some(Box::new(AuthEvent::SendOTPRequested {
            phone_number: self.phone_number.clone(),
        }))
    }

    fn read(&self, state: &RequestState) -> Option<Self::Data> {
        state.auth_result.clone()
    }
}

// OR: Edge can call action directly for simple cases (bypassing event flow)
pub async fn send_otp_direct(
    phone_number: String,
    ctx: &GraphQLContext,
) -> FieldResult<bool> {
    // Build an EffectContext from GraphQL context
    let effect_ctx = ctx.effect_context();
    let result = actions::send_otp(phone_number, &effect_ctx).await?;

    match result {
        AuthEvent::OTPSent { .. } => Ok(true),
        AuthEvent::PhoneNotRegistered { .. } => Ok(false),
        _ => Err(FieldError::new("unexpected result", Value::null())),
    }
}
```

**Benefits of Actions Pattern:**
| Aspect | Without Actions | With Actions |
|--------|----------------|--------------|
| Code location | Business logic in Effect | Business logic in Action |
| Reusability | Effect-only | Edges + Effects + Tests |
| Testing | Need to test via Effect | Test Action directly |
| Effect role | Does work | Thin dispatcher |
| Edge flexibility | Must go through event flow | Can call action directly |

**When to Use Actions:**
- ✅ Any IO operation (database, API calls, file operations)
- ✅ Business logic that could be called from multiple places
- ✅ Complex operations that benefit from isolation
- ❌ Pure state transitions (use Reducers instead)
- ❌ Simple event forwarding (keep in Effect)

**Action Naming Conventions:**
```rust
// Verb + noun, matches the operation
pub async fn send_otp(...) -> Result<AuthEvent>
pub async fn verify_otp(...) -> Result<AuthEvent>
pub async fn register_member(...) -> Result<MemberEvent>
pub async fn generate_assessment(...) -> Result<DomainApprovalEvent>
pub async fn create_message(...) -> Result<ChatEvent>
pub async fn extract_posts_from_pages(...) -> Result<PostExtractionEvent>
```

---

## Implementation Phases

### Phase 1: Foundation & Simple Domains

**Files to create/modify:**
- `packages/server/Cargo.toml` - Update seesaw version
- `packages/server/src/domains/auth/` - Migrate Auth domain
- `packages/server/src/domains/website/` - Migrate Website domain

**Tasks:**

#### 1.1 Update Dependencies

```toml
# packages/server/Cargo.toml
seesaw_core = "0.3.0"
seesaw-testing = "0.3.0"
seesaw-job-postgres = "0.3.0"  # If version exists
```

#### 1.2 Migrate Auth Domain (Simplest)

**Delete:**
- `domains/auth/commands.rs`
- `domains/auth/machines.rs`

**Create `domains/auth/actions/mod.rs`:**
```rust
mod send_otp;
mod verify_otp;

pub use send_otp::send_otp;
pub use verify_otp::verify_otp;
```

**Create `domains/auth/actions/send_otp.rs`:**
```rust
use crate::domains::auth::events::AuthEvent;
use crate::server::ServerDeps;
use seesaw_core::EffectContext;
use anyhow::Result;

/// Send OTP to phone number via Twilio.
/// Returns OTPSent on success, PhoneNotRegistered if phone not found.
pub async fn send_otp(
    phone_number: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<AuthEvent> {
    // Check if phone is registered (for login flow)
    let member = Member::find_by_phone(&phone_number, &ctx.deps().db_pool).await?;

    if member.is_none() {
        return Ok(AuthEvent::PhoneNotRegistered { phone_number });
    }

    // Send OTP via Twilio
    let identifier = ctx.deps().twilio.send_otp(&phone_number).await?;

    Ok(AuthEvent::OTPSent {
        phone_number,
        identifier,
    })
}
```

**Create `domains/auth/actions/verify_otp.rs`:**
```rust
use crate::domains::auth::events::AuthEvent;
use crate::server::ServerDeps;
use seesaw_core::EffectContext;
use anyhow::Result;

/// Verify OTP code. Returns OTPVerified with session on success.
pub async fn verify_otp(
    identifier: String,
    code: String,
    phone_number: String,
    ctx: &EffectContext<ServerDeps>,
) -> Result<AuthEvent> {
    let valid = ctx.deps().twilio.verify_otp(&identifier, &code).await?;

    if !valid {
        return Ok(AuthEvent::OTPFailed {
            phone_number,
            reason: "Invalid code".to_string(),
        });
    }

    let member = Member::find_by_phone(&phone_number, &ctx.deps().db_pool)
        .await?
        .expect("member must exist if OTP was sent");

    let session = Session::create(member.id, &ctx.deps().db_pool).await?;

    Ok(AuthEvent::OTPVerified {
        member_id: member.id,
        session_token: session.token,
    })
}
```

**Modify `domains/auth/effects.rs` (thin dispatcher):**
```rust
use super::actions;

pub struct AuthEffect;

#[async_trait]
impl Effect<AuthEvent, ServerDeps> for AuthEffect {
    type Event = AuthEvent;

    async fn handle(
        &mut self,
        event: AuthEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<AuthEvent> {
        match event {
            AuthEvent::SendOTPRequested { phone_number } => {
                actions::send_otp(phone_number, &ctx).await
            }
            AuthEvent::VerifyOTPRequested { identifier, code, phone_number } => {
                actions::verify_otp(identifier, code, phone_number, &ctx).await
            }
        }
    }
}
```

**Modify `domains/auth/mod.rs`:**
```rust
pub mod actions;  // NEW
pub mod effects;
pub mod events;
pub mod edges;
// DELETE: pub mod commands;
// DELETE: pub mod machines;
```

#### 1.3 Migrate Website Domain

Similar pattern to Auth - straightforward request/response without state.

#### 1.4 Update EngineBuilder Registration

**Modify `server/app.rs`:**

```rust
// BEFORE
.with_machine(AuthMachine)
.with_effect::<AuthCommand, _>(AuthEffect)

// AFTER
.with_effect::<AuthEvent, _>(AuthEffect)
```

---

### Phase 2: Stateful Domains

**Domains:** Member, Domain Approval

#### 2.1 Migrate Member Domain

**Challenge:** `MemberMachine` tracks `pending_registrations: HashMap<String, ()>`.

**Solution:** Move state to Effect (since Effects now have `&mut self`).

```rust
pub struct MemberEffect {
    pending_registrations: HashMap<String, ()>,
}

impl MemberEffect {
    pub fn new() -> Self {
        Self { pending_registrations: HashMap::new() }
    }
}

impl Effect<MemberEvent, ServerDeps> for MemberEffect {
    type Event = MemberEvent;

    async fn handle(&mut self, event: MemberEvent, ctx: EffectContext<ServerDeps>) -> Result<MemberEvent> {
        match event {
            MemberEvent::RegisterMemberRequested { idempotency_key, .. } => {
                self.pending_registrations.insert(idempotency_key.clone(), ());
                actions::register_member(event, &ctx).await
            }
            MemberEvent::MemberRegistered { idempotency_key, member_id, .. } => {
                self.pending_registrations.remove(&idempotency_key);
                // Schedule embedding generation as background job (typed)
                ctx.deps().job_queue.enqueue(GenerateEmbeddingJob { member_id }).await?;
                Ok(MemberEvent::EmbeddingScheduled { member_id })
            }
        }
    }
}
```

#### 2.2 Migrate Domain Approval Domain

**Challenge:** `DomainApprovalMachine` tracks requesters across multi-step async workflow.

**Solution:** Move HashMap to Effect, extract business logic to actions.

**Create `domains/domain_approval/actions/mod.rs`:**
```rust
mod fetch_or_create_research;
mod conduct_research_searches;
mod generate_assessment;

pub use fetch_or_create_research::fetch_or_create_research;
pub use conduct_research_searches::conduct_research_searches;
pub use generate_assessment::generate_assessment;
```

**Create `domains/domain_approval/actions/fetch_or_create_research.rs`:**
```rust
use crate::domains::domain_approval::events::DomainApprovalEvent;
use crate::server::ServerDeps;
use seesaw_core::EffectContext;
use anyhow::Result;

/// Fetch existing research or create new research for domain.
pub async fn fetch_or_create_research(
    domain_id: DomainId,
    job_id: JobId,
    requested_by: MemberId,
    ctx: &EffectContext<ServerDeps>,
) -> Result<DomainApprovalEvent> {
    // Check for existing recent research
    let existing = DomainResearch::find_latest_by_domain_id(
        domain_id.into(),
        &ctx.deps().db_pool,
    ).await?;

    if let Some(research) = existing {
        let age_days = (Utc::now() - research.created_at).num_days();
        if age_days < 7 {
            return Ok(DomainApprovalEvent::WebsiteResearchFound {
                research_id: research.id,
                domain_id,
                job_id,
                age_days,
            });
        }
    }

    // Scrape homepage and create new research
    let homepage = ctx.deps().firecrawl.scrape(&domain_id.to_url()).await
        .unwrap_or_default();  // Graceful failure

    let research = DomainResearch::create(
        domain_id,
        homepage,
        &ctx.deps().db_pool,
    ).await?;

    Ok(DomainApprovalEvent::WebsiteResearchCreated {
        research_id: research.id,
        domain_id,
        job_id,
    })
}
```

**Modify `domains/domain_approval/effects/mod.rs` (thin dispatcher with state):**
```rust
use super::actions;

pub struct DomainApprovalEffect {
    requesters: HashMap<Uuid, MemberId>,  // Tracks who requested each job
}

impl DomainApprovalEffect {
    pub fn new() -> Self {
        Self { requesters: HashMap::new() }
    }
}

#[async_trait]
impl Effect<DomainApprovalEvent, ServerDeps> for DomainApprovalEffect {
    type Event = DomainApprovalEvent;

    async fn handle(
        &mut self,
        event: DomainApprovalEvent,
        ctx: EffectContext<ServerDeps>,
    ) -> Result<Self::Event> {
        match event {
            DomainApprovalEvent::AssessWebsiteRequested { domain_id, job_id, requested_by } => {
                self.requesters.insert(job_id, requested_by);
                actions::fetch_or_create_research(domain_id, job_id, requested_by, &ctx).await
            }

            DomainApprovalEvent::WebsiteResearchCreated { research_id, domain_id, job_id } => {
                actions::conduct_research_searches(research_id, domain_id, job_id, &ctx).await
            }

            DomainApprovalEvent::ResearchSearchesCompleted { research_id, domain_id, job_id }
            | DomainApprovalEvent::WebsiteResearchFound { research_id, domain_id, job_id, .. } => {
                let requested_by = self.requesters.get(&job_id).copied()
                    .unwrap_or(MemberId::system());
                actions::generate_assessment(research_id, domain_id, job_id, requested_by, &ctx).await
            }

            DomainApprovalEvent::WebsiteAssessmentCompleted { job_id, assessment_id, .. } => {
                self.requesters.remove(&job_id);
                Ok(DomainApprovalEvent::WorkflowCompleted { job_id, assessment_id })
            }
        }
    }
}
```

---

### Phase 3: Complex Multi-Machine Workflows (Chat)

**Challenge:** Chat domain has 4 machines that chain together:
1. `ChatEventMachine` - Routes requests to commands
2. `AgentReplyMachine` - Observes MessageCreated, triggers reply generation
3. `AgentMessagingMachine` - Converts ReplyGenerated to CreateMessage
4. `GenerateAgentGreetingEffect` - Generates greeting on container creation

**Solution:** Consolidate into fewer effects that handle the full chain.

#### Current Flow (0.1.1):
```
SendMessageRequested
  → ChatEventMachine → ChatCommand::CreateMessage
  → ChatEffect → MessageCreated
  → AgentReplyMachine → GenerateChatReplyCommand
  → GenerateChatReplyEffect → ReplyGenerated
  → AgentMessagingMachine → ChatCommand::CreateMessage
  → ChatEffect → MessageCreated (assistant)
```

#### New Flow (0.3.0):
```
SendMessageRequested
  → ChatEffect.handle() → MessageCreated
  → AgentReplyEffect.handle(MessageCreated) → ReplyGenerated
  → ChatEffect.handle(ReplyGenerated) → MessageCreated (assistant)
```

**Create `domains/chatrooms/actions/mod.rs`:**
```rust
mod create_container;
mod create_message;
mod generate_reply;

pub use create_container::create_container;
pub use create_message::create_message;
pub use generate_reply::generate_reply;
```

**Create `domains/chatrooms/actions/create_message.rs`:**
```rust
use crate::domains::chatrooms::events::ChatEvent;
use crate::server::ServerDeps;
use seesaw_core::EffectContext;
use anyhow::Result;

/// Create a message in a container.
pub async fn create_message(
    container_id: Uuid,
    content: String,
    role: Role,
    sender: Option<MemberId>,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ChatEvent> {
    let message = Message::create(
        container_id,
        &content,
        role,
        sender,
        &ctx.deps().db_pool,
    ).await?;

    // Publish to NATS for real-time delivery
    ctx.deps().nats.publish(
        format!("containers.{}.messages", container_id),
        &message,
    ).await?;

    Ok(ChatEvent::MessageCreated {
        message_id: message.id,
        container_id,
        role,
    })
}
```

**Create `domains/chatrooms/actions/generate_reply.rs`:**
```rust
use crate::domains::chatrooms::events::ChatMessagingEvent;
use crate::server::ServerDeps;
use seesaw_core::EffectContext;
use anyhow::Result;

/// Generate AI reply for a container's conversation.
pub async fn generate_reply(
    container_id: Uuid,
    ctx: &EffectContext<ServerDeps>,
) -> Result<ChatMessagingEvent> {
    // Load conversation history
    let messages = Message::find_by_container(container_id, &ctx.deps().db_pool).await?;

    // Generate reply
    let reply = ctx.deps().ai.complete(&messages).await?;

    Ok(ChatMessagingEvent::ReplyGenerated {
        container_id,
        content: reply,
    })
}
```

**Implementation (Effects as thin dispatchers):**

```rust
// domains/chatrooms/effects/chat.rs
use super::super::actions;

pub struct ChatEffect;

#[async_trait]
impl Effect<ChatEvent, ServerDeps> for ChatEffect {
    type Event = ChatEvent;

    async fn handle(&mut self, event: ChatEvent, ctx: EffectContext<ServerDeps>) -> Result<ChatEvent> {
        match event {
            ChatEvent::SendMessageRequested { container_id, content, sender } => {
                actions::create_message(container_id, content, Role::User, Some(sender), &ctx).await
            }
            ChatEvent::CreateContainerRequested { member_id, with_agent } => {
                actions::create_container(member_id, with_agent, &ctx).await
            }
        }
    }
}

// Separate effect for ReplyGenerated → creates assistant message
pub struct ReplyToMessageEffect;

#[async_trait]
impl Effect<ChatMessagingEvent, ServerDeps> for ReplyToMessageEffect {
    type Event = ChatEvent;

    async fn handle(&mut self, event: ChatMessagingEvent, ctx: EffectContext<ServerDeps>) -> Result<ChatEvent> {
        let ChatMessagingEvent::ReplyGenerated { container_id, content } = event;
        actions::create_message(container_id, content, Role::Assistant, None, &ctx).await
    }
}
```

```rust
// domains/chatrooms/effects/agent_reply.rs
use super::super::actions;

pub struct AgentReplyEffect;

#[async_trait]
impl Effect<ChatEvent, ServerDeps> for AgentReplyEffect {
    type Event = ChatMessagingEvent;

    async fn handle(&mut self, event: ChatEvent, ctx: EffectContext<ServerDeps>) -> Result<ChatMessagingEvent> {
        let ChatEvent::MessageCreated { container_id, role: Role::User, .. } = event else {
            // Only trigger on user messages
            unreachable!("effect only registered for user MessageCreated")
        };
        actions::generate_reply(container_id, &ctx).await
    }
}
```

---

### Phase 4: Cross-Domain Event Routing

**Challenge:** `PostExtractionMachine` in Posts domain listens to `CrawlEvent` from Crawling domain.

**Solution:** Register Effect for the event type it needs, regardless of domain:

```rust
// In posts/extraction/effects.rs
pub struct PostExtractionEffect;

impl Effect<CrawlEvent, ServerDeps> for PostExtractionEffect {
    type Event = PostExtractionEvent;

    async fn handle(&mut self, event: CrawlEvent, ctx: EffectContext<ServerDeps>) -> Result<PostExtractionEvent> {
        let CrawlEvent::PagesReadyForExtraction { website_id, job_id, page_snapshot_ids } = event;
        actions::extract_posts_from_snapshots(website_id, job_id, page_snapshot_ids, &ctx).await
    }
}

// In app.rs registration:
EngineBuilder::new(server_deps)
    .with_effect::<CrawlEvent, _>(CrawlerEffect)
    .with_effect::<CrawlEvent, _>(PostExtractionEffect)  // Cross-domain listener
    .with_effect::<PostExtractionEvent, _>(PostSyncEffect)
    .build()
```

---

## Acceptance Criteria

### Functional Requirements

- [ ] All domains compile with seesaw 0.3.0
- [ ] Auth flow works: SendOTPRequested → OTPSent/PhoneNotRegistered
- [ ] Member registration works: RegisterMemberRequested → MemberRegistered → EmbeddingGenerated
- [ ] Chat messaging works: SendMessageRequested → MessageCreated → ReplyGenerated → MessageCreated
- [ ] Domain approval works: AssessWebsiteRequested → (research) → (search) → WebsiteAssessmentCompleted
- [ ] Cross-domain events work: CrawlEvent::PagesReadyForExtraction triggers PostExtractionEffect
- [ ] Background jobs continue to function

### Non-Functional Requirements

- [ ] No machines/ or commands/ directories remain
- [ ] Each domain has an `actions/` directory with business logic
- [ ] Effects are thin dispatchers (match event → call action → return result)
- [ ] All Effect implementations use new `handle(&mut self, event)` signature
- [ ] Stateful Effects properly manage in-memory state
- [ ] EngineBuilder uses `.with_effect::<EventType, _>()` pattern
- [ ] Existing tests pass (after updating to new patterns)
- [ ] Actions are unit-testable independently of Effects

### Quality Gates

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no errors
- [ ] Manual testing of each domain workflow

---

## Dependencies & Prerequisites

1. **seesaw-core 0.3.0** must be published to crates.io (or use git dependency)
2. **seesaw-testing** compatible version
3. **seesaw-job-postgres** compatible version (if using background jobs)

---

## Risk Analysis & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Multi-step chains break | High | Test each domain flow manually before moving to next |
| State tracking bugs | High | Add logging to state mutations during migration |
| Cross-domain events fail | Medium | Test CrawlEvent → PostExtractionEvent flow explicitly |
| Background jobs break | Medium | Verify job queue integration pattern early |
| Test suite failures | Low | Update tests incrementally per domain |

---

## Migration Order

1. **Auth** (simplest, no state, no deps)
2. **Website** (simple, no state)
3. **Member** (stateful, has background jobs)
4. **Domain Approval** (stateful, multi-step)
5. **Chat** (complex, 4-machine chain)
6. **Posts** (cross-domain receiver)
7. **Crawling** (cross-domain emitter)

---

## File Changes Summary

### Delete (per domain)
```
domains/{domain}/commands.rs (or commands/mod.rs)
domains/{domain}/machines.rs (or machines/mod.rs)
```

### Create (per domain)
```
domains/{domain}/actions/mod.rs        - Action module exports
domains/{domain}/actions/{action}.rs   - One file per action (send_otp.rs, verify_otp.rs, etc.)
domains/{domain}/jobs/mod.rs           - Job module exports (if domain has background jobs)
domains/{domain}/jobs/{job}.rs         - One file per job type (generate_embedding.rs, etc.)
```

### Modify
```
domains/{domain}/effects.rs - Thin dispatcher calling actions
domains/{domain}/mod.rs     - Add actions, remove commands/machines
server/app.rs               - Update EngineBuilder
Cargo.toml                  - Update seesaw version
```

### Tests to Update
```
domains/{domain}/tests/*.rs - Update to new patterns, test actions directly
```

---

## References

### Internal References
- Current architecture: `/Users/craig/Developer/fourthplaces/mntogether/docs/architecture/SEESAW_ARCHITECTURE.md`
- Recent refactor plan: `/Users/craig/Developer/fourthplaces/mntogether/docs/plans/2026-02-01-refactor-untangle-seesaw-architecture-plan.md`
- Engine setup: `/Users/craig/Developer/fourthplaces/mntogether/packages/server/src/server/app.rs:163-195`

### Seesaw 0.3.0 API (from README)
- Effect trait: `handle(&mut self, event, ctx) -> Result<Event>`
- EngineBuilder: `.with_effect::<EventType, _>(effect)`
- Reducers: `fn reduce(&self, state, event) -> State`
- Edges: `execute() -> Option<Event>`, `read(state) -> Option<Data>`

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        Event Bus                                 │
└─────────────────────────────────────────────────────────────────┘
       ▲                    │                    │
       │                    ▼                    ▼
   ┌───┴───┐          ┌─────────┐          ┌─────────┐
   │ Edge  │          │ Effect  │          │  Job    │
   │       │          │  (thin) │          │ Worker  │
   └───────┘          └────┬────┘          └────┬────┘
       │                   │                    │
       │                   ▼                    ▼
       │              ┌─────────┐          ┌─────────┐
       └─────────────►│ Action  │◄─────────│   Job   │
                      │         │          │         │
                      └────┬────┘          └────┬────┘
                           │                    │
                           ▼                    ▼
                      ┌─────────┐          ┌─────────┐
                      │  Event  │          │  Event  │
                      └────┬────┘          └────┬────┘
                           │                    │
                           └────────┬───────────┘
                                    ▼
                           (back to Event Bus)
```

**Flow:**
1. **Edge** emits initial event to bus
2. **Effect** (registered for that event) calls **Action**
3. **Action** does work, returns **Event**
4. **Event** emitted to bus, triggers next Effect (or settles)
5. **Jobs** run async, also call **Actions**, emit **Events**
