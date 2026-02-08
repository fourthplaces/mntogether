//! Lightweight SSE server for streaming events to clients.
//!
//! Subscribes to StreamHub topics and forwards events as SSE.
//! Runs alongside the Restate workflow server on a separate port.

use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::stream_hub::StreamHub;

/// Shared state for the SSE server.
#[derive(Clone)]
pub struct SseState {
    pub stream_hub: StreamHub,
}

/// Build the axum router for SSE endpoints.
pub fn router(state: SseState) -> Router {
    Router::new()
        .route("/api/streams/{topic}", get(stream_handler))
        .with_state(state)
}

/// SSE handler — subscribes to a StreamHub topic and streams events.
async fn stream_handler(
    State(state): State<SseState>,
    Path(topic): Path<String>,
) -> impl IntoResponse {
    let rx = state.stream_hub.subscribe(&topic).await;

    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(value) => {
            let event_type = value
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("message");

            Some(Ok::<_, Infallible>(
                Event::default()
                    .event(event_type)
                    .data(value.to_string()),
            ))
        }
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => {
            Some(Ok(Event::default().event("lagged").data("{}")))
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
