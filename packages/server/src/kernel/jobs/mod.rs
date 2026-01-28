//! Job infrastructure for background command execution.
//!
//! This module provides the kernel-level infrastructure for job execution:
//! - [`PostgresJobQueue`] - Database-backed job queue
//! - [`SeesawJobQueueAdapter`] - Adapter for seesaw's `JobQueue` trait
//! - [`JobWorker`] - Long-running service that polls and executes jobs
//! - [`Job`] - Job model with CRUD operations
//!
//! # Architecture
//!
//! ```text
//! Effect calls kernel.enqueue_job(cmd)
//!     │
//!     └─► SeesawJobQueueAdapter.enqueue()
//!             └─► Insert to DB (no NATS)
//!
//! JobWorker
//!     │
//!     ├─► Poll DB (claim jobs via JobStore)
//!     ├─► Deserialize command from JSON (CommandRegistry)
//!     ├─► Dispatcher.dispatch_one(command)
//!     │       └─► Effect.execute(cmd, EffectContext)
//!     └─► Mark succeeded/failed
//! ```
//!
//! # Domain-Specific Background Commands
//!
//! Background commands and effects live in their respective domains:
//! - `domains/agent/commands/messaging.rs` and `domains/agent/effects/messaging.rs` - Agent messaging
//! - `domains/entry/commands/background.rs` and `domains/entry/effects/background.rs` - Entry analysis
//! - `domains/deck/commands/background.rs` and `domains/deck/effects/background.rs` - Deck card generation
//! - `domains/container/commands/` and `domains/container/effects/` - Container-specific commands
//!
//! This module only provides the infrastructure - business logic stays in domains.

pub mod events;
mod job;
mod job_store;
pub mod manager;
mod queue;
mod seesaw_adapter;
pub mod testing;
mod worker;

pub use events::JobEvent;
pub use job::{ErrorKind, Job, JobPriority, JobStatus, MisfirePolicy, OverlapPolicy};
pub use job_store::PostgresJobStore;
pub use manager::{DefaultJobManager, JobManager, MockJobHandler, ScheduleOptions, TestJobManager};
pub use queue::{ClaimedJob, CommandMeta, EnqueueResult, JobQueue, PostgresJobQueue};
pub use seesaw_adapter::SeesawJobQueueAdapter;
pub use worker::{JobWorker, JobWorkerConfig, SeesawCommandHandler};

/// Register all background commands with the command registry.
///
/// This function registers deserializers for all background commands across all domains,
/// enabling the `JobWorker` to deserialize job payloads back to typed commands.
///
/// Call this during job worker startup.
///
/// NOTE: Currently commented out as this codebase doesn't use the job queue system yet.
/// Uncomment and adapt when background job processing is needed.
#[allow(dead_code)]
pub fn register_commands(_registry: &mut seesaw::CommandRegistry) {
    // TODO: Register organization domain background commands when needed
    // use crate::domains::organization::commands::{...};
    // registry.register::<SomeBackgroundCommand>("job_type", vec![1]);
}
