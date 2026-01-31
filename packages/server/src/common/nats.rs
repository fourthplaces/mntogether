//! Common NATS publishing utilities.
//!
//! This module provides a unified pattern for publishing domain events to NATS.
//! Events implement the `IntoNatsPayload` trait to declare their NATS behavior.
//!
//! # Publishing Mechanism
//!
//! NATS publishing is handled by event taps (see `nats_tap` module), which run
//! after effects complete. This ensures events are published only after state
//! has been committed.

use uuid::Uuid;

// =============================================================================
// IntoNatsPayload Trait
// =============================================================================

/// Trait for domain events that can be published to NATS.
///
/// Implement this trait on your domain events to enable automatic NATS publishing.
/// Events that return `None` from `container_id()` will not be published.
///
/// # Example
///
/// ```ignore
/// impl IntoNatsPayload for MyEvent {
///     fn container_id(&self) -> Option<Uuid> {
///         match self {
///             MyEvent::Created { container_id, .. } => Some(*container_id),
///             MyEvent::Deleted { .. } => None, // Don't publish deletions
///         }
///     }
///
///     fn into_payload(&self) -> serde_json::Value {
///         serde_json::to_value(self).unwrap()
///     }
///
///     fn subject_suffix() -> &'static str {
///         "my_domain"
///     }
/// }
/// ```
pub trait IntoNatsPayload: Send + Sync {
    /// Get the container ID for scoping the publish to container members.
    ///
    /// Return `None` if this event should not be published to NATS.
    fn container_id(&self) -> Option<Uuid>;

    /// Convert the event to a NATS-serializable JSON payload.
    fn into_payload(&self) -> serde_json::Value;

    /// Get the NATS subject suffix (e.g., "messages", "typing").
    ///
    /// The full subject will be: `members.{member_id}.containers.{container_id}.{suffix}`
    fn subject_suffix() -> &'static str;

    /// Optional: Get an additional container ID to publish to (e.g., chat container).
    ///
    /// Override this to publish to multiple containers. Default returns `None`.
    fn additional_container_id(&self) -> Option<Uuid> {
        None
    }

    /// Optional: Get a member ID to exclude from publishing (e.g., the sender for typing events).
    ///
    /// Override this to skip specific members. Default returns `None`.
    fn exclude_member_id(&self) -> Option<Uuid> {
        None
    }
}
