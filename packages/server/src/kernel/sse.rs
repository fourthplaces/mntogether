//! Lightweight SSE server for streaming events to clients.
//!
//! Subscribes to StreamHub topics and forwards events as SSE.
//! Runs alongside the Restate workflow server on a separate port.
//! Requires a valid JWT token via `?token=` query parameter.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::domains::auth::JwtService;

use super::stream_hub::StreamHub;

/// Shared state for the SSE server.
#[derive(Clone)]
pub struct SseState {
    pub stream_hub: StreamHub,
    pub jwt_service: Arc<JwtService>,
}

/// Build the axum router for SSE endpoints.
pub fn router(state: SseState) -> Router {
    Router::new()
        .route("/api/streams/{topic}", get(stream_handler))
        .with_state(state)
}

/// SSE handler — validates JWT from query param, then subscribes to a StreamHub topic.
async fn stream_handler(
    State(state): State<SseState>,
    Path(topic): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Require valid JWT token via query parameter
    let token = match params.get("token") {
        Some(t) if !t.is_empty() => t,
        _ => return (StatusCode::UNAUTHORIZED, "Token required").into_response(),
    };

    if state.jwt_service.verify_token(token).is_err() {
        return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    }

    let rx = state.stream_hub.subscribe(&topic).await;

    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(value) => {
            let event_type = value
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("message");

            Some(Ok::<_, Infallible>(
                Event::default().event(event_type).data(value.to_string()),
            ))
        }
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => {
            Some(Ok(Event::default().event("lagged").data("{}")))
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}
