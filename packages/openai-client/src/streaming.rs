//! SSE streaming parser for OpenAI chat completions.
//!
//! Converts a raw `reqwest` byte stream into `ChatCompletionChunk` values.
//! Handles `data: [DONE]`, partial lines, and buffering.

use bytes::Bytes;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::OpenAIError;

/// A single chunk from a streaming chat completion.
#[derive(Debug, Clone)]
pub struct ChatCompletionChunk {
    /// The text delta for this chunk.
    pub delta: String,
    /// Whether the stream is done.
    pub done: bool,
}

/// Raw streaming chunk from OpenAI API.
#[derive(Debug, serde::Deserialize)]
struct StreamChunkRaw {
    choices: Vec<StreamChoiceRaw>,
}

#[derive(Debug, serde::Deserialize)]
struct StreamChoiceRaw {
    delta: DeltaRaw,
}

#[derive(Debug, serde::Deserialize)]
struct DeltaRaw {
    #[serde(default)]
    content: Option<String>,
}

/// Stream adapter that converts raw SSE bytes into `ChatCompletionChunk` values.
pub struct ChatCompletionStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
    buffer: String,
}

impl ChatCompletionStream {
    pub(crate) fn new(
        byte_stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    ) -> Self {
        Self {
            inner: Box::pin(byte_stream),
            buffer: String::new(),
        }
    }
}

impl Stream for ChatCompletionStream {
    type Item = Result<ChatCompletionChunk, OpenAIError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            // Try to parse a complete line from the buffer
            if let Some(chunk) = try_parse_line(&mut this.buffer) {
                return Poll::Ready(Some(chunk));
            }

            // Need more data from the byte stream
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => this.buffer.push_str(text),
                        Err(e) => {
                            return Poll::Ready(Some(Err(OpenAIError::Parse(format!(
                                "Invalid UTF-8 in stream: {}",
                                e
                            )))));
                        }
                    }
                    // Loop to try parsing again
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(OpenAIError::Network(e.to_string()))));
                }
                Poll::Ready(None) => {
                    // Stream ended — check for remaining buffer content
                    if this.buffer.trim().is_empty() {
                        return Poll::Ready(None);
                    }
                    // Try to parse any remaining content
                    if let Some(chunk) = try_parse_line(&mut this.buffer) {
                        return Poll::Ready(Some(chunk));
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Try to extract and parse a complete SSE line from the buffer.
/// Returns `None` if no complete line is available yet.
fn try_parse_line(buffer: &mut String) -> Option<Result<ChatCompletionChunk, OpenAIError>> {
    loop {
        let newline_pos = buffer.find('\n')?;
        let line = buffer[..newline_pos].trim().to_string();
        buffer.drain(..=newline_pos);

        // Skip empty lines (SSE uses blank lines as event separators)
        if line.is_empty() {
            continue;
        }

        // Handle SSE data lines
        if let Some(data) = line.strip_prefix("data: ") {
            let data = data.trim();

            // Done signal
            if data == "[DONE]" {
                return Some(Ok(ChatCompletionChunk {
                    delta: String::new(),
                    done: true,
                }));
            }

            // Parse JSON chunk
            match serde_json::from_str::<StreamChunkRaw>(data) {
                Ok(raw) => {
                    let delta = raw
                        .choices
                        .into_iter()
                        .next()
                        .and_then(|c| c.delta.content)
                        .unwrap_or_default();

                    return Some(Ok(ChatCompletionChunk { delta, done: false }));
                }
                Err(e) => {
                    return Some(Err(OpenAIError::Parse(format!(
                        "Failed to parse stream chunk: {} (data: {})",
                        e,
                        &data[..data.len().min(200)]
                    ))));
                }
            }
        }

        // Skip non-data lines (e.g., "event:", "id:", "retry:")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    fn make_sse_bytes(lines: &[&str]) -> Vec<Result<Bytes, reqwest::Error>> {
        lines
            .iter()
            .map(|line| Ok(Bytes::from(format!("{}\n", line))))
            .collect()
    }

    #[tokio::test]
    async fn test_parse_single_chunk() {
        let data = make_sse_bytes(&[
            r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#,
            "",
            "data: [DONE]",
        ]);

        let byte_stream = futures::stream::iter(data);
        let mut stream = ChatCompletionStream::new(byte_stream);

        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.delta, "Hello");
        assert!(!chunk.done);

        let done = stream.next().await.unwrap().unwrap();
        assert!(done.done);
    }

    #[tokio::test]
    async fn test_parse_multiple_tokens() {
        let data = make_sse_bytes(&[
            r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#,
            "",
            r#"data: {"choices":[{"delta":{"content":" world"}}]}"#,
            "",
            "data: [DONE]",
        ]);

        let byte_stream = futures::stream::iter(data);
        let mut stream = ChatCompletionStream::new(byte_stream);

        let c1 = stream.next().await.unwrap().unwrap();
        assert_eq!(c1.delta, "Hello");

        let c2 = stream.next().await.unwrap().unwrap();
        assert_eq!(c2.delta, " world");

        let done = stream.next().await.unwrap().unwrap();
        assert!(done.done);
    }

    #[tokio::test]
    async fn test_empty_delta() {
        let data = make_sse_bytes(&[r#"data: {"choices":[{"delta":{}}]}"#, "", "data: [DONE]"]);

        let byte_stream = futures::stream::iter(data);
        let mut stream = ChatCompletionStream::new(byte_stream);

        let chunk = stream.next().await.unwrap().unwrap();
        assert_eq!(chunk.delta, "");
    }
}
