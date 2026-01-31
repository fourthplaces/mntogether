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
//! Background commands and effects live in their respective domains.
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
