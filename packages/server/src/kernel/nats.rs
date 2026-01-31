//! NATS client abstraction for production and testing.
//!
//! Provides a trait-based NATS implementation that allows swapping between
//! real NATS connections and test mocks.

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::RwLock;

/// A published message.
#[derive(Debug, Clone)]
pub struct PublishedMessage {
    pub subject: String,
    pub payload: Bytes,
}

/// Trait for NATS publish operations.
///
/// This allows swapping between real NATS and test mocks.
#[async_trait]
pub trait NatsPublisher: Send + Sync {
    /// Publish a message to a subject.
    async fn publish(&self, subject: String, payload: Bytes) -> Result<()>;
}

/// Real NATS client publisher.
pub struct NatsClientPublisher {
    client: async_nats::Client,
}

impl NatsClientPublisher {
    pub fn new(client: async_nats::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NatsPublisher for NatsClientPublisher {
    async fn publish(&self, subject: String, payload: Bytes) -> Result<()> {
        self.client.publish(subject, payload).await?;
        Ok(())
    }
}

/// Mock NATS client that tracks published messages for testing.
///
/// This allows tests to inspect what messages would have been published
/// to NATS without requiring a real connection.
#[derive(Default)]
pub struct TestNats {
    /// Messages published to subjects.
    published: RwLock<Vec<PublishedMessage>>,
    /// Subscriptions that were created.
    subscriptions: RwLock<Vec<String>>,
}

impl TestNats {
    /// Create a new test NATS client.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a published message.
    pub fn record_publish(&self, subject: String, payload: Bytes) {
        self.published
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(PublishedMessage { subject, payload });
    }

    /// Record a subscription.
    pub fn record_subscription(&self, subject: String) {
        self.subscriptions
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .push(subject);
    }

    /// Get all published messages.
    pub fn published_messages(&self) -> Vec<PublishedMessage> {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Get published messages for a specific subject.
    pub fn messages_for_subject(&self, subject: &str) -> Vec<PublishedMessage> {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|m| m.subject == subject)
            .cloned()
            .collect()
    }

    /// Get published messages matching a subject prefix.
    pub fn messages_with_prefix(&self, prefix: &str) -> Vec<PublishedMessage> {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|m| m.subject.starts_with(prefix))
            .cloned()
            .collect()
    }

    /// Check if any message was published to a subject.
    pub fn was_published_to(&self, subject: &str) -> bool {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|m| m.subject == subject)
    }

    /// Check if any message was published with a subject prefix.
    pub fn was_published_with_prefix(&self, prefix: &str) -> bool {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|m| m.subject.starts_with(prefix))
    }

    /// Get all subscribed subjects.
    pub fn subscriptions(&self) -> Vec<String> {
        self.subscriptions
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Check if a subscription was created for a subject.
    pub fn was_subscribed_to(&self, subject: &str) -> bool {
        self.subscriptions
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&subject.to_string())
    }

    /// Get the count of published messages.
    pub fn publish_count(&self) -> usize {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Get the count of messages published to a specific subject.
    pub fn publish_count_for(&self, subject: &str) -> usize {
        self.published
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|m| m.subject == subject)
            .count()
    }

    /// Clear all recorded messages and subscriptions.
    pub fn clear(&self) {
        self.published
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.subscriptions
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Get messages grouped by subject.
    pub fn messages_by_subject(&self) -> HashMap<String, Vec<PublishedMessage>> {
        let messages = self.published.read().unwrap_or_else(|e| e.into_inner());
        let mut by_subject: HashMap<String, Vec<PublishedMessage>> = HashMap::new();

        for msg in messages.iter() {
            by_subject
                .entry(msg.subject.clone())
                .or_default()
                .push(msg.clone());
        }

        by_subject
    }

    /// Deserialize a published message payload as JSON.
    pub fn deserialize_message<T: serde::de::DeserializeOwned>(
        &self,
        msg: &PublishedMessage,
    ) -> std::result::Result<T, serde_json::Error> {
        serde_json::from_slice(&msg.payload)
    }
}

#[async_trait]
impl NatsPublisher for TestNats {
    async fn publish(&self, subject: String, payload: Bytes) -> Result<()> {
        self.record_publish(subject, payload);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_retrieve_messages() {
        let nats = TestNats::new();

        nats.record_publish(
            "chat.messages".to_string(),
            Bytes::from(r#"{"id":"123"}"#),
        );

        assert_eq!(nats.publish_count(), 1);
        assert!(nats.was_published_to("chat.messages"));
        assert!(!nats.was_published_to("chat.other"));
    }

    #[test]
    fn test_messages_with_prefix() {
        let nats = TestNats::new();

        nats.record_publish("chat.messages".to_string(), Bytes::new());
        nats.record_publish("chat.typing".to_string(), Bytes::new());
        nats.record_publish("members.push".to_string(), Bytes::new());

        assert_eq!(nats.messages_with_prefix("chat.").len(), 2);
        assert_eq!(nats.messages_with_prefix("members.").len(), 1);
        assert!(nats.was_published_with_prefix("chat."));
    }

    #[test]
    fn test_clear() {
        let nats = TestNats::new();

        nats.record_publish("test".to_string(), Bytes::new());
        nats.record_subscription("test.>".to_string());

        assert_eq!(nats.publish_count(), 1);
        assert_eq!(nats.subscriptions().len(), 1);

        nats.clear();

        assert_eq!(nats.publish_count(), 0);
        assert_eq!(nats.subscriptions().len(), 0);
    }
}
