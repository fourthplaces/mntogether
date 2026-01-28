use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// Expo Push Notification Client
/// Sends push notifications to Expo Go mobile app users
pub struct ExpoClient {
    client: Client,
    access_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExpoMessage {
    to: String,
    title: String,
    body: String,
    data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    sound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    badge: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ExpoResponse {
    data: Vec<ExpoTicket>,
}

#[derive(Debug, Deserialize)]
struct ExpoTicket {
    status: String,
    #[allow(dead_code)]
    id: Option<String>,
    #[allow(dead_code)]
    message: Option<String>,
    #[allow(dead_code)]
    details: Option<serde_json::Value>,
}

impl ExpoClient {
    pub fn new(access_token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            access_token,
        }
    }

    /// Send a push notification to an Expo push token
    pub async fn send_notification(
        &self,
        push_token: &str,
        title: &str,
        body: &str,
        data: serde_json::Value,
    ) -> Result<()> {
        let message = ExpoMessage {
            to: push_token.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            data,
            sound: Some("default".to_string()),
            badge: None,
        };

        let mut request = self
            .client
            .post("https://exp.host/--/api/v2/push/send")
            .json(&message);

        // Add access token if provided (for higher rate limits)
        if let Some(token) = &self.access_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        info!("Sending Expo push notification to: {}", push_token);

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            error!("Expo push failed {}: {}", status, body);
            anyhow::bail!("Expo push API error {}: {}", status, body);
        }

        let expo_response: ExpoResponse = response.json().await?;

        // Check for errors in the response
        for ticket in &expo_response.data {
            if ticket.status == "error" {
                error!("Expo ticket error: {:?}", ticket);
                anyhow::bail!("Expo ticket error: {:?}", ticket);
            }
        }

        info!("Expo notification sent successfully");
        Ok(())
    }

    /// Send multiple notifications in batch (up to 100)
    pub async fn send_batch(
        &self,
        notifications: Vec<(&str, &str, &str, serde_json::Value)>,
    ) -> Result<()> {
        if notifications.is_empty() {
            return Ok(());
        }

        let messages: Vec<ExpoMessage> = notifications
            .into_iter()
            .map(|(token, title, body, data)| ExpoMessage {
                to: token.to_string(),
                title: title.to_string(),
                body: body.to_string(),
                data,
                sound: Some("default".to_string()),
                badge: None,
            })
            .collect();

        let mut request = self
            .client
            .post("https://exp.host/--/api/v2/push/send")
            .json(&messages);

        if let Some(token) = &self.access_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        info!(
            "Sending batch of {} Expo push notifications",
            messages.len()
        );

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await?;
            error!("Expo batch push failed {}: {}", status, body);
            anyhow::bail!("Expo push API error {}: {}", status, body);
        }

        let expo_response: ExpoResponse = response.json().await?;

        // Log any errors but don't fail the whole batch
        let mut error_count = 0;
        for ticket in &expo_response.data {
            if ticket.status == "error" {
                error!("Expo ticket error: {:?}", ticket);
                error_count += 1;
            }
        }

        if error_count > 0 {
            error!(
                "{} out of {} notifications failed",
                error_count,
                expo_response.data.len()
            );
        } else {
            info!(
                "All {} notifications sent successfully",
                expo_response.data.len()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expo_client_creation() {
        let client = ExpoClient::new(None);
        assert!(client.access_token.is_none());

        let client_with_token = ExpoClient::new(Some("test-token".to_string()));
        assert!(client_with_token.access_token.is_some());
    }

    #[tokio::test]
    #[ignore] // Requires valid Expo push token
    async fn test_send_notification() {
        let client = ExpoClient::new(None);
        let token = std::env::var("TEST_EXPO_TOKEN").expect("TEST_EXPO_TOKEN not set");

        let result = client
            .send_notification(
                &token,
                "Test Notification",
                "This is a test message",
                serde_json::json!({"test": true}),
            )
            .await;

        assert!(result.is_ok());
    }
}
