//! Generic in-process pub/sub hub for real-time streaming.
//!
//! Provides topic-keyed broadcast channels for pushing events to SSE endpoints.
//! Topics are opaque strings — the hub has no knowledge of what's being streamed.
//!
//! # Usage
//!
//! Producers (domain actions):
//!   hub.publish("chat:abc-123", json!({"type": "token_delta", "delta": "Hello"})).await;
//!
//! Consumers (SSE endpoints):
//!   let rx = hub.subscribe("chat:abc-123").await;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Generic in-process pub/sub hub.
///
/// Thread-safe, cloneable. Keyed by string topics.
/// Payloads are `serde_json::Value` — domains serialize their own types.
#[derive(Clone)]
pub struct StreamHub {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<serde_json::Value>>>>,
    capacity: usize,
}

impl StreamHub {
    /// Create a new StreamHub with default capacity (256 messages per channel).
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    /// Create a new StreamHub with the given channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    /// Publish a JSON value to a topic. No-op if no subscribers.
    pub async fn publish(&self, topic: &str, value: serde_json::Value) {
        let channels = self.channels.read().await;
        if let Some(tx) = channels.get(topic) {
            // Ignore send errors (no active receivers)
            let _ = tx.send(value);
        }
    }

    /// Subscribe to a topic. Creates the channel if it doesn't exist.
    pub async fn subscribe(&self, topic: &str) -> broadcast::Receiver<serde_json::Value> {
        let mut channels = self.channels.write().await;
        let tx = channels
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(self.capacity).0);
        tx.subscribe()
    }

    /// Remove channels with zero subscribers (housekeeping).
    pub async fn cleanup(&self) {
        let mut channels = self.channels.write().await;
        channels.retain(|_, tx| tx.receiver_count() > 0);
    }
}

impl Default for StreamHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_subscribe_roundtrip() {
        let hub = StreamHub::new();
        let mut rx = hub.subscribe("test:topic").await;

        let value = serde_json::json!({"type": "token_delta", "delta": "hello"});
        hub.publish("test:topic", value.clone()).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received, value);
    }

    #[tokio::test]
    async fn test_publish_no_subscribers_is_noop() {
        let hub = StreamHub::new();
        // Should not panic
        hub.publish("nobody:listening", serde_json::json!({"data": "dropped"}))
            .await;
    }

    #[tokio::test]
    async fn test_cleanup_removes_empty_channels() {
        let hub = StreamHub::new();
        let rx = hub.subscribe("ephemeral:topic").await;

        assert_eq!(hub.channels.read().await.len(), 1);

        drop(rx);
        hub.cleanup().await;

        assert_eq!(hub.channels.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let hub = StreamHub::new();
        let mut rx1 = hub.subscribe("multi:topic").await;
        let mut rx2 = hub.subscribe("multi:topic").await;

        let value = serde_json::json!({"type": "broadcast"});
        hub.publish("multi:topic", value.clone()).await;

        assert_eq!(rx1.recv().await.unwrap(), value);
        assert_eq!(rx2.recv().await.unwrap(), value);
    }
}
