//! Chatrooms domain - containers and messages for AI chat, comments, discussions.
//!
//! Architecture (seesaw Edge pattern):
//!   Edge.execute() → Request Event → Effect → Fact Event → Reducer → Edge.read()
//!
//! Components:
//! - events: Request events (user intent) and fact events (what happened)
//! - actions: Business logic functions
//! - effects: Handles request events, calls actions, emits fact events
//! - reducer: Stores fact event results in state for Edge.read()
//! - state: Per-request state for Edge pattern
//! - edges: Edge structs (CreateChat, SendMessage)
//! - edges/internal: React to fact events, emit new request events
//! - edges/query: GraphQL queries (read-only)

pub mod actions;
pub mod data;
pub mod edges;
pub mod effects;
pub mod events;
pub mod models;
pub mod reducer;
pub mod state;

// Re-export commonly used types
pub use data::*;
pub use effects::ChatEffect;
pub use events::ChatEvent;
pub use models::*;
pub use reducer::ChatReducer;
pub use state::ChatRequestState;
