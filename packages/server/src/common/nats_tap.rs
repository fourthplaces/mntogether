//! NATS Event Taps - observe events and publish to NATS.
//!
//! This module provides event taps for publishing domain events to NATS
//! after effects complete. Taps are observers - they don't make decisions
//! or mutate state, just publish committed facts to external systems.
//!
//! # Usage
//!
//! Register taps with an engine:
//!
//! ```ignore
//! use crate::common::nats_tap::NatsPublishTap;
//!
//! EngineBuilder::with_arc(kernel.clone())
//!     .with_bus(bus)
//!     .with_event_tap(NatsPublishTap::<ChatEvent>::new(nats_publisher.clone()))
//!     .build()
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use seesaw_core::{async_trait, EventTap, TapContext};
use tracing::{info, warn};

use crate::common::nats::IntoNatsPayload;
use crate::kernel::nats::NatsPublisher;

// =============================================================================
// NATS Publish Tap
// =============================================================================

/// Event tap that publishes events to NATS after effects complete.
///
/// This tap observes events that implement `IntoNatsPayload` and publishes
/// them to all members of the event's container(s).
///
/// # Guarantees
///
/// - Runs after effects complete (observes committed facts)
/// - Fire-and-forget (errors are logged, not propagated)
/// - Does not block the main event loop
///
/// # Example
///
/// ```ignore
/// let tap = NatsPublishTap::<ChatEvent>::new(nats_publisher.clone());
/// engine_builder.with_event_tap(tap);
/// ```
pub struct NatsPublishTap<E> {
    nats: Arc<dyn NatsPublisher>,
    _phantom: PhantomData<E>,
}

impl<E> NatsPublishTap<E> {
    /// Create a new NATS publish tap.
    pub fn new(nats: Arc<dyn NatsPublisher>) -> Self {
        Self {
            nats,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<E> EventTap<E> for NatsPublishTap<E>
where
    E: IntoNatsPayload + Clone + Send + Sync + 'static,
{
    async fn on_event(&self, event: &E, _ctx: &TapContext) -> Result<()> {
        publish_event(event, &*self.nats).await
    }
}

// =============================================================================
// Publishing Logic
// =============================================================================

/// Publish an event to NATS.
///
/// This function:
/// 1. Gets the container ID from the event
/// 2. Serializes the event payload
/// 3. Publishes to the container's NATS subject
///
/// If the event returns `None` from `container_id()`, nothing is published.
///
/// # Subject Format
///
/// `containers.{container_id}.{suffix}`
///
/// For example: `containers.abc123.messages`
async fn publish_event<E: IntoNatsPayload>(event: &E, nats: &dyn NatsPublisher) -> Result<()> {
    let Some(container_id) = event.container_id() else {
        return Ok(()); // Event doesn't want to be published
    };

    let payload = event.into_payload();
    let suffix = E::subject_suffix();

    // Publish to main container subject
    let subject = format!("containers.{}.{}", container_id, suffix);
    let payload_bytes = serde_json::to_vec(&payload)?;

    info!(
        container_id = %container_id,
        subject = %subject,
        "publishing NATS event"
    );

    if let Err(e) = nats.publish(subject.clone(), Bytes::from(payload_bytes.clone())).await {
        warn!(
            error = %e,
            subject = %subject,
            "Failed to publish NATS event"
        );
    }

    // Publish to additional container if specified
    if let Some(additional_id) = event.additional_container_id() {
        if additional_id != container_id {
            let additional_subject = format!("containers.{}.{}", additional_id, suffix);

            if let Err(e) = nats
                .publish(additional_subject.clone(), Bytes::from(payload_bytes))
                .await
            {
                warn!(
                    error = %e,
                    subject = %additional_subject,
                    "Failed to publish NATS event to additional container"
                );
            }
        }
    }

    Ok(())
}

// =============================================================================
// Simple Broadcast (for typing indicators without container lookup)
// =============================================================================

/// Publish an event directly to a container subject.
///
/// Use this for ephemeral events like typing indicators that don't need
/// member-scoped delivery.
pub async fn broadcast_to_container<E: IntoNatsPayload>(
    event: &E,
    nats: &dyn NatsPublisher,
) -> Result<()> {
    publish_event(event, nats).await
}
