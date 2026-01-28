# Chat Architecture - Volunteer Intake Conversations

## Overview

This document defines the real-time chat system for volunteer intake conversations, following Shay's proven patterns but adapted for mndigitalaid.

**Key Differences from Shay:**
- ✅ **Redis Pub/Sub** (not NATS) for message broadcasting across multiple servers
- ✅ **Decoupled participation** via `room_participants` relation table
- ✅ **Privacy-first** - rooms linked to volunteers via UUID, not PII
- ✅ **AI-assisted intake** - rig.rs generates conversational responses
- ✅ **GraphQL subscriptions** for real-time updates

**Use Case:**
When volunteers register, they enter a conversational intake room where an AI agent asks clarifying questions about their skills, availability, and interests. This creates richer `searchable_text` profiles than a single text field.

---

## Database Schema

### Rooms Table

```sql
-- Rooms: Conversation spaces for volunteer intake
CREATE TABLE rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Type of room (intake, support, admin_chat)
    room_type TEXT NOT NULL DEFAULT 'intake',

    -- Room metadata
    status TEXT DEFAULT 'active',  -- active, archived, closed

    -- Last activity tracking
    last_message_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_rooms_status ON rooms(status) WHERE status = 'active';
CREATE INDEX idx_rooms_last_message ON rooms(last_message_at DESC);
```

### Room Participants Table (Decoupled Relation)

```sql
-- Room participants: Decouples volunteers from rooms
-- Allows volunteers to be in multiple rooms, rooms to have multiple participants
CREATE TABLE room_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,

    -- Participant can be volunteer or system/admin
    volunteer_id UUID REFERENCES volunteers(id) ON DELETE CASCADE,

    -- Role in room (participant, moderator, ai_agent)
    role TEXT DEFAULT 'participant',

    -- Notification preferences for this room
    muted BOOLEAN DEFAULT false,

    -- Tracking
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    last_read_at TIMESTAMPTZ,  -- For unread count

    UNIQUE(room_id, volunteer_id)
);

CREATE INDEX idx_room_participants_volunteer ON room_participants(volunteer_id);
CREATE INDEX idx_room_participants_room ON room_participants(room_id);
```

### Messages Table

```sql
-- Messages: Individual chat messages
CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,

    -- Author (NULL for system messages)
    volunteer_id UUID REFERENCES volunteers(id) ON DELETE SET NULL,

    -- Message content
    text TEXT NOT NULL,

    -- Message type (user, ai, system)
    message_type TEXT DEFAULT 'user',

    -- Threading support (optional - for replies)
    reply_to_id UUID REFERENCES messages(id) ON DELETE SET NULL,

    -- AI metadata (if AI-generated)
    ai_prompt TEXT,  -- Prompt used to generate this message
    ai_model TEXT,   -- Model used (e.g., "gpt-4o")
    ai_tokens_used INTEGER,  -- Token count for cost tracking

    -- Status (draft, sent, edited, deleted)
    status TEXT DEFAULT 'sent',

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_messages_room ON messages(room_id, created_at DESC);
CREATE INDEX idx_messages_volunteer ON messages(volunteer_id);
CREATE INDEX idx_messages_reply_to ON messages(reply_to_id);
```

### Typing Indicators (Optional - In-Memory via Redis)

**NOT stored in database** - ephemeral state via Redis with TTL.

```redis
# Key format: typing:{room_id}:{volunteer_id}
# Value: timestamp
# TTL: 10 seconds

SET typing:550e8400-e29b-41d4-a716-446655440000:7c9e6679-7425-40de-944b-e07fc1f90ae7 "2026-01-27T18:30:00Z" EX 10
```

---

## Redis Pub/Sub Architecture

### Why Redis Instead of NATS?

**Shay uses NATS:**
- Distributed messaging system
- Requires separate NATS server
- Subject-based routing

**We use Redis:**
- ✅ Already using Redis for caching
- ✅ Simpler deployment (one less service)
- ✅ Pub/Sub sufficient for MVP scale
- ✅ Built-in TTL for typing indicators

### Redis Channels

```
rooms:{room_id}:messages     # Message events (created, updated, deleted)
rooms:{room_id}:typing       # Typing indicators
rooms:global                 # Global announcements (optional)
```

### Event Payloads

**Message Event:**
```json
{
  "type": "message_created",
  "message": {
    "id": "uuid",
    "room_id": "uuid",
    "volunteer_id": "uuid",
    "text": "Hello, I'd like to volunteer!",
    "message_type": "user",
    "created_at": "2026-01-27T18:30:00Z"
  }
}
```

**Typing Event:**
```json
{
  "type": "typing_started",
  "room_id": "uuid",
  "volunteer_id": "uuid",
  "timestamp": "2026-01-27T18:30:00Z"
}
```

---

## Package Structure

Following `PACKAGE_STRUCTURE.md`, add new domains:

```
src/domains/
├── room/                    # Room management
│   ├── commands/
│   │   ├── create.rs        # CreateRoom
│   │   ├── archive.rs       # ArchiveRoom
│   │   └── close.rs         # CloseRoom
│   ├── data/
│   │   └── types.rs         # RoomInput
│   ├── edges/
│   │   ├── query.rs         # Query resolvers
│   │   └── mutation.rs      # Mutation resolvers
│   ├── effects/
│   │   └── db_effects.rs    # Database operations
│   ├── events/
│   │   └── types.rs         # RoomCreated, RoomArchived
│   ├── models/
│   │   ├── room.rs          # Room struct
│   │   └── participant.rs   # RoomParticipant struct
│   ├── errors.rs
│   ├── mod.rs
│   └── registry.rs
│
└── message/                 # Chat messaging
    ├── commands/
    │   ├── send.rs          # SendMessage
    │   ├── edit.rs          # EditMessage
    │   ├── delete.rs        # DeleteMessage
    │   └── typing.rs        # SendTypingIndicator
    ├── data/
    │   └── types.rs         # MessageInput
    ├── edges/
    │   ├── query.rs
    │   ├── mutation.rs
    │   └── subscription.rs  # ⭐ GraphQL subscriptions
    ├── effects/
    │   ├── db_effects.rs    # Database operations
    │   ├── ai_effects.rs    # AI response generation (rig.rs)
    │   └── redis_effects.rs # Redis pub/sub
    ├── events/
    │   └── types.rs         # MessageSent, TypingStarted
    ├── machines/
    │   ├── intake.rs        # Intake conversation flow
    │   └── ai_response.rs   # AI response generation
    ├── models/
    │   └── message.rs       # Message struct
    ├── realtime/            # ⭐ Real-time streaming
    │   ├── mod.rs
    │   ├── events.rs        # RoomEvent enum
    │   └── stream.rs        # subscribe_to_room()
    ├── prompts/
    │   ├── intake.rs        # Intake conversation prompts
    │   └── clarification.rs # Follow-up question prompts
    ├── errors.rs
    ├── mod.rs
    └── registry.rs
```

---

## Core Types

### Rust Models

```rust
// src/domains/room/models/room.rs

use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Room {
    pub id: Uuid,
    pub room_type: String,  // intake, support, admin_chat
    pub status: String,     // active, archived, closed
    pub last_message_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// src/domains/room/models/participant.rs

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoomParticipant {
    pub id: Uuid,
    pub room_id: Uuid,
    pub volunteer_id: Option<Uuid>,
    pub role: String,  // participant, moderator, ai_agent
    pub muted: bool,
    pub joined_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
}

// src/domains/message/models/message.rs

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub room_id: Uuid,
    pub volunteer_id: Option<Uuid>,
    pub text: String,
    pub message_type: String,  // user, ai, system
    pub reply_to_id: Option<Uuid>,

    // AI metadata
    pub ai_prompt: Option<String>,
    pub ai_model: Option<String>,
    pub ai_tokens_used: Option<i32>,

    pub status: String,  // draft, sent, edited, deleted
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### GraphQL Schema

```graphql
# ═══════════════════════════════════════════════════════════════════════
#                      TYPES
# ═══════════════════════════════════════════════════════════════════════

type Room {
  id: ID!
  roomType: String!
  status: String!
  lastMessageAt: DateTime
  participants: [RoomParticipant!]!
  messages(limit: Int = 50, cursor: ID): MessageConnection!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type RoomParticipant {
  id: ID!
  roomId: ID!
  volunteerId: ID
  role: String!
  muted: Boolean!
  joinedAt: DateTime!
  lastReadAt: DateTime
}

type Message {
  id: ID!
  roomId: ID!
  volunteerId: ID
  text: String!
  messageType: String!
  replyToId: ID

  # AI metadata (if AI-generated)
  aiPrompt: String
  aiModel: String
  aiTokensUsed: Int

  status: String!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type MessageConnection {
  nodes: [Message!]!
  pageInfo: PageInfo!
}

# ═══════════════════════════════════════════════════════════════════════
#                      QUERIES
# ═══════════════════════════════════════════════════════════════════════

extend type Query {
  # Get volunteer's rooms
  myRooms(limit: Int = 20): [Room!]!

  # Get specific room
  room(id: ID!): Room

  # Get messages for a room (paginated)
  messages(roomId: ID!, limit: Int = 50, cursor: ID): MessageConnection!
}

# ═══════════════════════════════════════════════════════════════════════
#                      MUTATIONS
# ═══════════════════════════════════════════════════════════════════════

extend type Mutation {
  # Create intake room (auto-created on volunteer registration)
  createIntakeRoom: Room!

  # Send message
  sendMessage(input: SendMessageInput!): Message!

  # Edit message (user only, within 5 min)
  editMessage(messageId: ID!, text: String!): Message!

  # Delete message (soft delete)
  deleteMessage(messageId: ID!): Boolean!

  # Send typing indicator (ephemeral, Redis only)
  sendTypingIndicator(roomId: ID!): Boolean!

  # Mark messages as read
  markRoomAsRead(roomId: ID!): Boolean!

  # Leave room
  leaveRoom(roomId: ID!): Boolean!
}

# ═══════════════════════════════════════════════════════════════════════
#                      SUBSCRIPTIONS (⭐ Real-time)
# ═══════════════════════════════════════════════════════════════════════

type Subscription {
  # Subscribe to room events (messages, typing)
  roomEvents(roomId: ID!): RoomEvent!
}

# ═══════════════════════════════════════════════════════════════════════
#                      INPUTS
# ═══════════════════════════════════════════════════════════════════════

input SendMessageInput {
  roomId: ID!
  text: String!
  replyToId: ID  # Optional - for threading
}

# ═══════════════════════════════════════════════════════════════════════
#                      SUBSCRIPTION EVENTS
# ═══════════════════════════════════════════════════════════════════════

type RoomEvent {
  eventType: String!  # message_created, message_updated, message_deleted, typing_started
  message: Message
  typingVolunteerId: ID
}
```

---

## Real-Time Streaming Implementation

### Redis Pub/Sub Client

```rust
// src/common/redis/mod.rs

use anyhow::Result;
use redis::{Client, aio::Connection, AsyncCommands};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct RedisClient {
    client: Client,
    connection: Arc<RwLock<Connection>>,
}

impl RedisClient {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        let connection = client.get_async_connection().await?;

        Ok(Self {
            client,
            connection: Arc::new(RwLock::new(connection)),
        })
    }

    /// Publish event to Redis channel
    pub async fn publish<T: Serialize>(
        &self,
        channel: &str,
        event: &T,
    ) -> Result<()> {
        let payload = serde_json::to_string(event)?;
        let mut conn = self.connection.write().await;
        conn.publish(channel, payload).await?;
        Ok(())
    }

    /// Subscribe to Redis channel
    pub async fn subscribe(&self, channel: String) -> Result<redis::aio::PubSub> {
        let mut pubsub = self.client.get_async_connection().await?.into_pubsub();
        pubsub.subscribe(channel).await?;
        Ok(pubsub)
    }
}
```

### Room Event Stream

```rust
// src/domains/message/realtime/events.rs

use serde::{Deserialize, Serialize};
use crate::domains::message::models::Message;

/// Events streamed to clients for a room
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RoomEvent {
    MessageCreated {
        message: MessageEventData,
    },
    MessageUpdated {
        message: MessageEventData,
    },
    MessageDeleted {
        message_id: String,
    },
    TypingStarted {
        room_id: String,
        volunteer_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEventData {
    pub id: String,
    pub room_id: String,
    pub volunteer_id: Option<String>,
    pub text: String,
    pub message_type: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Message> for MessageEventData {
    fn from(message: Message) -> Self {
        Self {
            id: message.id.to_string(),
            room_id: message.room_id.to_string(),
            volunteer_id: message.volunteer_id.map(|id| id.to_string()),
            text: message.text,
            message_type: message.message_type,
            created_at: message.created_at.to_rfc3339(),
            updated_at: message.updated_at.to_rfc3339(),
        }
    }
}
```

### Subscription Handler

```rust
// src/domains/message/realtime/stream.rs

use super::events::RoomEvent;
use crate::common::redis::RedisClient;
use anyhow::Result;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use tracing::{debug, warn};
use uuid::Uuid;

pub type RoomEventStream = Pin<Box<dyn Stream<Item = RoomEvent> + Send>>;

/// Subscribe to real-time events for a room.
///
/// Verifies the volunteer has access to the room before subscribing.
/// Returns a stream of `RoomEvent` for the given room.
/// Events are sourced from Redis channels:
/// - `rooms:{room_id}:messages` - message events
/// - `rooms:{room_id}:typing` - typing events
pub async fn subscribe_to_room(
    redis: &RedisClient,
    volunteer_id: Uuid,
    room_id: Uuid,
    pool: &sqlx::PgPool,
) -> Result<RoomEventStream> {
    // Verify volunteer is participant of room
    let participant = sqlx::query!(
        "SELECT id FROM room_participants WHERE room_id = $1 AND volunteer_id = $2",
        room_id,
        volunteer_id
    )
    .fetch_optional(pool)
    .await?;

    if participant.is_none() {
        anyhow::bail!("Volunteer is not a participant of this room");
    }

    let channel = format!("rooms:{}:messages", room_id);

    debug!(
        volunteer_id = %volunteer_id,
        room_id = %room_id,
        channel = %channel,
        "Subscribing to room events"
    );

    let mut pubsub = redis.subscribe(channel.clone()).await?;

    let stream = async_stream::stream! {
        while let Some(msg) = pubsub.on_message().next().await {
            let payload: String = match msg.get_payload() {
                Ok(p) => p,
                Err(e) => {
                    warn!(err = %e, "Failed to get Redis payload");
                    continue;
                }
            };

            match serde_json::from_str::<RoomEvent>(&payload) {
                Ok(event) => yield event,
                Err(e) => {
                    warn!(err = %e, payload = %payload, "Failed to parse room event");
                }
            }
        }
    };

    Ok(Box::pin(stream))
}
```

### GraphQL Subscription Resolver

```rust
// src/domains/message/edges/subscription.rs

use crate::domains::message::realtime::{RoomEvent, subscribe_to_room};
use crate::server::graphql::context::Context;
use juniper::FieldError;
use std::pin::Pin;
use uuid::Uuid;

pub struct SubscriptionRoot;

type RoomEventStream =
    Pin<Box<dyn futures::Stream<Item = Result<RoomEvent, FieldError>> + Send>>;

#[juniper::graphql_subscription(Context = Context)]
impl SubscriptionRoot {
    /// Subscribe to events for a room.
    /// Receives message changes and typing indicators.
    async fn room_events(context: &Context, room_id: String) -> RoomEventStream {
        let volunteer_id = match context.volunteer_id {
            Some(id) => id,
            None => {
                return Box::pin(futures::stream::once(async {
                    Err(FieldError::new(
                        "Authentication required",
                        juniper::Value::null(),
                    ))
                }));
            }
        };

        let room_uuid = match Uuid::parse_str(&room_id) {
            Ok(id) => id,
            Err(_) => {
                return Box::pin(futures::stream::once(async {
                    Err(FieldError::new(
                        "Invalid room ID",
                        juniper::Value::null(),
                    ))
                }));
            }
        };

        let redis = context.redis.clone();
        let pool = context.pool.clone();

        let stream = async_stream::stream! {
            match subscribe_to_room(&redis, volunteer_id, room_uuid, &pool).await {
                Ok(mut event_stream) => {
                    use futures::StreamExt;
                    while let Some(event) = event_stream.next().await {
                        yield Ok(event);
                    }
                }
                Err(e) => {
                    yield Err(FieldError::new(
                        format!("Subscription failed: {}", e),
                        juniper::Value::null(),
                    ));
                }
            }
        };

        Box::pin(stream)
    }
}
```

---

## AI-Assisted Intake

### Intake Conversation Flow

```rust
// src/domains/message/machines/intake.rs

use crate::domains::message::prompts::intake::INTAKE_PROMPT;
use crate::common::ai::RigClient;
use anyhow::Result;

pub struct IntakeMachine {
    rig: RigClient,
}

impl IntakeMachine {
    pub fn new(rig: RigClient) -> Self {
        Self { rig }
    }

    /// Generate AI response for intake conversation
    pub async fn generate_response(
        &self,
        conversation_history: &[Message],
        latest_message: &str,
    ) -> Result<String> {
        // Build conversation context
        let context = conversation_history
            .iter()
            .map(|msg| {
                let role = if msg.message_type == "user" { "Human" } else { "AI" };
                format!("{}: {}", role, msg.text)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "{}\n\nConversation so far:\n{}\n\nHuman: {}\n\nAI:",
            INTAKE_PROMPT,
            context,
            latest_message
        );

        let response = self.rig.complete(&prompt).await?;
        Ok(response)
    }
}
```

### Intake Prompts

```rust
// src/domains/message/prompts/intake.rs

pub const INTAKE_PROMPT: &str = r#"You are a friendly intake assistant helping volunteers register for community service opportunities.

Your goal is to understand:
1. What skills, experience, or interests they have
2. When they're available to volunteer
3. Where they're located or willing to travel
4. Any specific causes or communities they want to help

Ask 1-2 clarifying questions at a time. Keep responses conversational and warm.

Example questions:
- "What kind of volunteering interests you most?"
- "Do you have any specific skills you'd like to use? (like translation, legal knowledge, medical training, etc.)"
- "Are there particular days or times that work best for you?"
- "Are you located in the Twin Cities area, or willing to travel?"

Once you have a good understanding (after 3-5 exchanges), thank them and let them know they'll start receiving relevant opportunities."#;
```

### AI Response Effect

```rust
// src/domains/message/effects/ai_effects.rs

use crate::common::ai::RigClient;
use crate::domains::message::models::Message;
use crate::domains::message::machines::intake::IntakeMachine;
use anyhow::Result;
use uuid::Uuid;
use sqlx::PgPool;

pub async fn generate_ai_response(
    pool: &PgPool,
    rig: &RigClient,
    room_id: Uuid,
    latest_message: &Message,
) -> Result<Message> {
    // Fetch conversation history
    let history = sqlx::query_as!(
        Message,
        r#"
        SELECT * FROM messages
        WHERE room_id = $1
        ORDER BY created_at ASC
        LIMIT 20
        "#,
        room_id
    )
    .fetch_all(pool)
    .await?;

    // Generate AI response
    let machine = IntakeMachine::new(rig.clone());
    let response_text = machine.generate_response(&history, &latest_message.text).await?;

    // Save AI message
    let ai_message = sqlx::query_as!(
        Message,
        r#"
        INSERT INTO messages (room_id, volunteer_id, text, message_type, ai_prompt, ai_model)
        VALUES ($1, NULL, $2, 'ai', $3, 'gpt-4o')
        RETURNING *
        "#,
        room_id,
        response_text,
        latest_message.text
    )
    .fetch_one(pool)
    .await?;

    Ok(ai_message)
}
```

---

## Integration with Volunteer Registration

### Updated Registration Flow

```graphql
mutation RegisterVolunteer($input: RegisterVolunteerInput!) {
  registerVolunteer(input: $input) {
    volunteer {
      id
      searchableText
      expoPushToken
    }
    intakeRoom {
      id
      # Automatically created
    }
    firstMessage {
      # AI greeting message
      text
    }
  }
}
```

### Registration Effect (Updated)

```rust
// src/domains/volunteer/effects/register.rs

use crate::domains::volunteer::models::Volunteer;
use crate::domains::room::models::Room;
use crate::domains::message::models::Message;
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn register_volunteer_with_intake(
    pool: &PgPool,
    searchable_text: &str,
    expo_push_token: &str,
) -> Result<(Volunteer, Room, Message)> {
    let mut tx = pool.begin().await?;

    // 1. Create volunteer
    let volunteer = sqlx::query_as!(
        Volunteer,
        r#"
        INSERT INTO volunteers (searchable_text, expo_push_token)
        VALUES ($1, $2)
        RETURNING *
        "#,
        searchable_text,
        expo_push_token
    )
    .fetch_one(&mut *tx)
    .await?;

    // 2. Create intake room
    let room = sqlx::query_as!(
        Room,
        r#"
        INSERT INTO rooms (room_type, status)
        VALUES ('intake', 'active')
        RETURNING *
        "#
    )
    .fetch_one(&mut *tx)
    .await?;

    // 3. Add volunteer as participant
    sqlx::query!(
        r#"
        INSERT INTO room_participants (room_id, volunteer_id, role)
        VALUES ($1, $2, 'participant')
        "#,
        room.id,
        volunteer.id
    )
    .execute(&mut *tx)
    .await?;

    // 4. Send AI greeting
    let greeting = "Hi! I'm here to help you find volunteer opportunities that match your interests and availability. To get started, could you tell me a bit about what kind of volunteering you're interested in?";

    let first_message = sqlx::query_as!(
        Message,
        r#"
        INSERT INTO messages (room_id, volunteer_id, text, message_type, ai_model)
        VALUES ($1, NULL, $2, 'ai', 'gpt-4o')
        RETURNING *
        "#,
        room.id,
        greeting
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((volunteer, room, first_message))
}
```

---

## Deployment Considerations

### Redis Setup

```bash
# Add Redis to fly.toml
flyctl redis create --name mndigitalaid-redis --region ord

# Attach Redis
flyctl redis attach mndigitalaid-redis

# Sets REDIS_URL environment variable
```

### Cargo Dependencies

```toml
[dependencies]
# Redis pub/sub
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# Async streams for subscriptions
async-stream = "0.3"
futures = "0.3"
```

### Environment Variables

```bash
REDIS_URL=redis://localhost:6379  # or Fly Redis URL
```

---

## Summary

This chat architecture provides:

✅ **Real-time messaging** via Redis pub/sub + GraphQL subscriptions
✅ **Decoupled participation** via `room_participants` relation table
✅ **Privacy-first** - rooms linked to volunteers by UUID only
✅ **AI-assisted intake** using rig.rs (gpt-4o) for conversational profiles
✅ **Multi-server support** - Redis broadcast handles multiple Rust servers
✅ **Threading support** - `reply_to_id` for message threads (optional)
✅ **Typing indicators** - Ephemeral state via Redis TTL
✅ **Message editing** - Within 5 minutes of creation

**Next Steps:**
1. Add `rooms`, `room_participants`, `messages` migrations
2. Implement Redis pub/sub client in `src/common/redis/`
3. Build `message` domain with real-time streaming
4. Integrate intake room creation into volunteer registration
5. Implement AI response generation via rig.rs
6. Add GraphQL subscription support to server

**Cost Impact:**
- Redis hosting: ~$5-10/month (Fly.io Redis)
- AI intake conversations: ~$0.01-0.02 per volunteer (5-10 exchanges)
- Total additional cost: ~$10-20/month
