//! ReadResult - deferred database read after effects settle
//!
//! Actions return `ReadResult<T>` instead of `T` directly. The actual database
//! read is deferred until `.read()` is called, which happens after the seesaw
//! engine has processed all cascading effects.

use anyhow::{Context, Result};
use sqlx::PgPool;

use super::readable::Readable;

/// A deferred database read that executes after effects settle.
///
/// Actions return this instead of the model directly. GraphQL resolvers call
/// `.read()` after `engine.activate().process()` completes.
///
/// # Example
///
/// ```rust,ignore
/// // In action:
/// pub async fn register_member(
///     args: RegisterArgs,
///     ctx: &RunContext<AppState, ServerDeps>,
/// ) -> Result<ReadResult<Member>> {
///     let member = Member::create(&args, ctx.deps().db_pool()).await?;
///     ctx.emit(MemberEvent::MemberRegistered { member_id: member.id });
///     Ok(ReadResult::new(member.id, ctx.deps().db_pool().clone()))
/// }
///
/// // In GraphQL resolver:
/// let member = ctx.engine
///     .activate(AppState::default())
///     .process(|run_ctx| actions::register_member(args, run_ctx))
///     .await?
///     .read()
///     .await?;
/// ```
pub struct ReadResult<T: Readable> {
    id: T::Id,
    pool: PgPool,
}

impl<T: Readable> ReadResult<T> {
    /// Create a new deferred read result.
    ///
    /// The actual database read is deferred until `.read()` is called.
    pub fn new(id: T::Id, pool: PgPool) -> Self {
        Self { id, pool }
    }

    /// Execute the database read.
    ///
    /// This should be called after `engine.activate().process()` completes,
    /// ensuring all cascading effects have settled before reading.
    pub async fn read(self) -> Result<T> {
        T::read_by_id(self.id, &self.pool)
            .await?
            .context("Entity not found after action completed")
    }

    /// Get the ID without performing the read.
    pub fn id(&self) -> &T::Id {
        &self.id
    }
}
