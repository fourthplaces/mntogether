//! NATS Event Publishing - publish events to NATS subjects.
//!
//! This module provides utilities for publishing domain events to NATS.
//! In seesaw 0.5.0+, this replaces the EventTap pattern.
//!
//! # Usage
//!
//! ```ignore
//! use crate::common::nats_tap::broadcast_to_container;
//!
//! // Publish an event to NATS
//! broadcast_to_container(&event, &*nats_publisher).await?;
//! ```

use anyhow::Result;
use bytes::Bytes;
use tracing::{info, warn};

use crate::common::nats::IntoNatsPayload;
use crate::kernel::nats::NatsPublisher;

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
pub async fn publish_event<E: IntoNatsPayload>(event: &E, nats: &dyn NatsPublisher) -> Result<()> {
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

    if let Err(e) = nats
        .publish(subject.clone(), Bytes::from(payload_bytes.clone()))
        .await
    {
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
