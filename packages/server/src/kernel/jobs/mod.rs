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
pub fn register_commands(registry: &mut seesaw::CommandRegistry) {
    use crate::domains::agent::commands::{
        GENERATE_AGENT_GREETING_JOB_TYPE, GENERATE_AGENT_REPLY_JOB_TYPE,
        GenerateAgentGreetingCommand, GenerateAgentReplyCommand,
    };
    use crate::domains::container::commands::{
        CONTAINER_SUMMARY_JOB_TYPE, ContainerSummaryCommand,
    };
    use crate::domains::deck::commands::{
        GENERATE_DECK_CARDS_JOB_TYPE, GENERATE_DECK_TITLE_JOB_TYPE, GenerateDeckCardsCommand,
        GenerateDeckTitleCommand, REGENERATE_DECK_CARDS_JOB_TYPE, RegenerateDeckCardsCommand,
    };
    use crate::domains::entry::{
        AAD_MIGRATION_JOB_TYPE, ANALYZE_ENTRY_JOB_TYPE, AUTO_TAG_ENTRY_JOB_TYPE,
        AadMigrationCommand, AnalyzeEntryCommand, AutoTagEntryCommand,
    };

    // Agent domain
    registry.register::<GenerateAgentReplyCommand>(GENERATE_AGENT_REPLY_JOB_TYPE, vec![1]);
    registry.register::<GenerateAgentGreetingCommand>(GENERATE_AGENT_GREETING_JOB_TYPE, vec![1]);

    // Deck domain
    registry.register::<GenerateDeckCardsCommand>(GENERATE_DECK_CARDS_JOB_TYPE, vec![1]);
    registry.register::<GenerateDeckTitleCommand>(GENERATE_DECK_TITLE_JOB_TYPE, vec![1]);
    registry.register::<RegenerateDeckCardsCommand>(REGENERATE_DECK_CARDS_JOB_TYPE, vec![1]);

    // Entry domain
    registry.register::<AnalyzeEntryCommand>(ANALYZE_ENTRY_JOB_TYPE, vec![1]);
    registry.register::<AutoTagEntryCommand>(AUTO_TAG_ENTRY_JOB_TYPE, vec![1]);
    registry.register::<AadMigrationCommand>(AAD_MIGRATION_JOB_TYPE, vec![1]);

    // Container domain
    registry.register::<ContainerSummaryCommand>(CONTAINER_SUMMARY_JOB_TYPE, vec![1]);
}
