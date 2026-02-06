//! SSE streaming endpoint.
//!
//! GET /api/streams/:topic?token=JWT
//!
//! Generic SSE endpoint with query-param auth and topic-level authorization.
//! Subscribes to StreamHub by topic string and forwards JSON values as SSE events.
//!
//! Auth strategy: JWT passed as `?token=` query param.
//! EventSource can't send custom headers, and the auth cookie lives on the
//! Next.js domain (not the API domain), so cookies won't be sent cross-origin.
//! The token is read from `document.cookie` on the client and appended to the URL.

use std::convert::Infallible;

use axum::{
    extract::{Extension, Path, Query},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::{self, StreamExt};
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;

use crate::server::app::AxumAppState;

#[derive(Deserialize)]
pub struct StreamQuery {
    /// JWT token for authentication
    token: Option<String>,
}

/// SSE stream handler.
///
/// Auth: Reads JWT from `?token=` query param, falls back to Authorization header.
/// Topic authorization: Extracts domain from topic prefix, verifies access.
///
/// Special case: `public_chat:{container_id}` topics skip JWT auth entirely.
/// The container UUID serves as an unguessable access credential.
pub async fn stream_handler(
    Extension(state): Extension<AxumAppState>,
    Path(topic): Path<String>,
    Query(query): Query<StreamQuery>,
    headers: HeaderMap,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    // Determine the internal StreamHub topic to subscribe to
    let subscribe_topic = if let Some(container_id) = topic.strip_prefix("public_chat:") {
        // Public chat: no auth required, subscribe to the internal chat topic
        // Validate that it looks like a UUID to prevent abuse
        uuid::Uuid::parse_str(container_id).map_err(|_| StatusCode::BAD_REQUEST)?;
        format!("chat:{}", container_id)
    } else {
        // All other topics require JWT auth
        let token = query.token.or_else(|| extract_bearer_token(&headers));
        let token = token.ok_or(StatusCode::UNAUTHORIZED)?;

        let claims = state
            .jwt_service
            .verify_token(&token)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        authorize_topic(&topic, &claims).map_err(|_| StatusCode::FORBIDDEN)?;
        topic.clone()
    };

    // Subscribe to StreamHub
    let rx = state.stream_hub.subscribe(&subscribe_topic).await;

    // Stream with connected event and lag handling
    let connected =
        stream::once(async { Ok::<_, Infallible>(Event::default().event("connected").data("ok")) });

    let events = BroadcastStream::new(rx).filter_map(|result| async {
        match result {
            Ok(value) => {
                let event_name = value
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("message");
                Event::default()
                    .event(event_name)
                    .json_data(&value)
                    .ok()
                    .map(Ok)
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                Event::default()
                    .event("lagged")
                    .json_data(&serde_json::json!({"missed": n}))
                    .ok()
                    .map(Ok)
            }
        }
    });

    Ok(Sse::new(connected.chain(events)).keep_alive(KeepAlive::default()))
}

/// Extract Bearer token from Authorization header.
fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    auth.strip_prefix("Bearer ").map(|t| t.to_string())
}

/// Topic-level authorization.
///
/// Parses the topic prefix to determine the domain, then checks access.
fn authorize_topic(
    topic: &str,
    claims: &crate::domains::auth::jwt::Claims,
) -> Result<(), anyhow::Error> {
    if topic.starts_with("chat:") {
        // Chat topics: admin access required for now
        if !claims.is_admin {
            anyhow::bail!("Admin access required for chat streams");
        }
        Ok(())
    } else {
        anyhow::bail!("Unknown topic prefix: {}", topic)
    }
}
