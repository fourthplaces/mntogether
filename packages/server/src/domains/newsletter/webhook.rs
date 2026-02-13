//! Postmark inbound webhook handler for newsletter ingestion.
//!
//! Receives inbound emails from Postmark, routes them to the correct
//! newsletter subscription, and either:
//! - Stores them as extraction_pages (active subscriptions)
//! - Extracts confirmation links (pending_confirmation subscriptions)

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use crate::domains::source::models::NewsletterSource;
use crate::kernel::ServerDeps;

use super::email_parser;

/// State shared with the webhook handler.
#[derive(Clone)]
pub struct WebhookState {
    pub deps: Arc<ServerDeps>,
}

/// Postmark inbound email payload.
/// See: https://postmarkapp.com/developer/webhooks/inbound-webhook
#[derive(Debug, Deserialize)]
pub struct PostmarkInboundPayload {
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "FromFull")]
    pub from_full: Option<PostmarkAddress>,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "HtmlBody")]
    pub html_body: Option<String>,
    #[serde(rename = "TextBody")]
    pub text_body: Option<String>,
    #[serde(rename = "MessageID")]
    pub message_id: String,
    #[serde(rename = "Date")]
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct PostmarkAddress {
    #[serde(rename = "Email")]
    pub email: String,
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

/// Build the axum router for webhook endpoints.
pub fn router(state: WebhookState) -> Router {
    Router::new()
        .route("/webhooks/postmark/inbound", post(handle_postmark_inbound))
        .with_state(state)
}

/// Handle an inbound email from Postmark.
///
/// Always returns 200 OK to prevent Postmark from retrying.
/// Processing failures are logged but do not affect the response.
async fn handle_postmark_inbound(
    State(state): State<WebhookState>,
    Json(payload): Json<PostmarkInboundPayload>,
) -> StatusCode {
    // Always return 200 to Postmark regardless of processing outcome
    if let Err(e) = process_inbound_email(&state, &payload).await {
        tracing::error!(
            message_id = %payload.message_id,
            from = %payload.from,
            to = %payload.to,
            subject = %payload.subject,
            error = %e,
            "Failed to process inbound email"
        );
    }

    StatusCode::OK
}

/// Process an inbound email based on the subscription state.
async fn process_inbound_email(
    state: &WebhookState,
    payload: &PostmarkInboundPayload,
) -> anyhow::Result<()> {
    let pool = &state.deps.db_pool;

    // Extract the ingest email from the To field
    let ingest_email = extract_ingest_email(&payload.to);

    // Look up the newsletter source by ingest email
    let newsletter_source = match NewsletterSource::find_by_ingest_email(&ingest_email, pool).await?
    {
        Some(ns) => ns,
        None => {
            tracing::debug!(
                to = %payload.to,
                "No newsletter source found for ingest email, dropping"
            );
            return Ok(());
        }
    };

    match newsletter_source.subscription_status.as_str() {
        "pending_confirmation" => {
            handle_confirmation_email(state, &newsletter_source, payload).await
        }
        "active" => handle_newsletter_email(state, &newsletter_source, payload).await,
        status => {
            tracing::debug!(
                source_id = %newsletter_source.source_id,
                status = %status,
                "Dropping email for non-active subscription"
            );
            Ok(())
        }
    }
}

/// Handle a confirmation email: extract the confirmation link and surface it for admin review.
async fn handle_confirmation_email(
    state: &WebhookState,
    newsletter_source: &NewsletterSource,
    payload: &PostmarkInboundPayload,
) -> anyhow::Result<()> {
    let pool = &state.deps.db_pool;

    // Extract confirmation link from HTML body
    let confirmation_link = payload
        .html_body
        .as_deref()
        .and_then(email_parser::extract_confirmation_link)
        .or_else(|| {
            // Fallback: try text body
            payload
                .text_body
                .as_deref()
                .and_then(email_parser::extract_confirmation_link)
        });

    let sender_domain = email_parser::extract_sender_domain(&payload.from)
        .unwrap_or_else(|| "unknown".to_string());

    match confirmation_link {
        Some(link) => {
            tracing::info!(
                source_id = %newsletter_source.source_id,
                confirmation_link = %link,
                sender_domain = %sender_domain,
                "Confirmation link extracted from email"
            );
            NewsletterSource::set_confirmation_link(
                newsletter_source.id,
                &link,
                &sender_domain,
                pool,
            )
            .await?;
        }
        None => {
            tracing::warn!(
                source_id = %newsletter_source.source_id,
                subject = %payload.subject,
                "Could not extract confirmation link from email"
            );
        }
    }

    Ok(())
}

/// Handle an active newsletter email: convert to markdown and store as extraction_page.
async fn handle_newsletter_email(
    state: &WebhookState,
    newsletter_source: &NewsletterSource,
    payload: &PostmarkInboundPayload,
) -> anyhow::Result<()> {
    let pool = &state.deps.db_pool;

    // Validate sender domain
    if let Some(expected_domain) = &newsletter_source.expected_sender_domain {
        let sender_domain = email_parser::extract_sender_domain(&payload.from);
        match sender_domain {
            Some(ref domain) if email_parser::sender_domain_matches(domain, expected_domain) => {
                // Sender matches — proceed
            }
            _ => {
                tracing::warn!(
                    source_id = %newsletter_source.source_id,
                    from = %payload.from,
                    expected_domain = %expected_domain,
                    "Dropping email from unexpected sender domain"
                );
                return Ok(());
            }
        }
    }

    // Convert email content to markdown
    let content = if let Some(html) = &payload.html_body {
        email_parser::html_to_markdown(html)
    } else if let Some(text) = &payload.text_body {
        text.clone()
    } else {
        tracing::warn!(
            source_id = %newsletter_source.source_id,
            message_id = %payload.message_id,
            "Email has no HTML or text body"
        );
        return Ok(());
    };

    // Skip empty content
    if content.trim().len() < 50 {
        tracing::debug!(
            source_id = %newsletter_source.source_id,
            "Skipping newsletter with insufficient content"
        );
        return Ok(());
    }

    // Build extraction_page identifiers
    let site_url = format!("newsletter:{}", newsletter_source.source_id);
    let page_url = format!("newsletter:{}:{}", newsletter_source.source_id, payload.message_id);

    // Store as extraction_page using the extraction service
    if let Some(extraction) = state.deps.extraction.as_ref() {
        let cached_page = extraction::CachedPage::new(&page_url, &site_url, &content)
            .with_title(payload.subject.clone())
            .with_metadata("subject", &payload.subject)
            .with_metadata("from", &payload.from)
            .with_metadata("date", &payload.date)
            .with_metadata("message_id", &payload.message_id);

        extraction.store_page(&cached_page).await?;

        tracing::info!(
            source_id = %newsletter_source.source_id,
            page_url = %page_url,
            subject = %payload.subject,
            content_len = content.len(),
            "Stored newsletter as extraction_page"
        );
    } else {
        tracing::error!("Extraction service not available — cannot store newsletter");
        return Err(anyhow::anyhow!("Extraction service not configured"));
    }

    // Update newsletter received counter
    NewsletterSource::record_newsletter_received(newsletter_source.id, pool).await?;

    // TODO: Trigger post extraction workflow for this newsletter

    Ok(())
}

/// Extract the ingest email from a To field.
/// Handles formats like "Name <uuid@ingest.mntogether.org>" and plain "uuid@ingest.mntogether.org".
fn extract_ingest_email(to: &str) -> String {
    if let Some(start) = to.find('<') {
        if let Some(end) = to.find('>') {
            return to[start + 1..end].trim().to_lowercase();
        }
    }
    to.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ingest_email() {
        assert_eq!(
            extract_ingest_email("abc@ingest.mntogether.org"),
            "abc@ingest.mntogether.org"
        );
        assert_eq!(
            extract_ingest_email("Newsletter <abc@ingest.mntogether.org>"),
            "abc@ingest.mntogether.org"
        );
        assert_eq!(
            extract_ingest_email("  ABC@INGEST.MNTOGETHER.ORG  "),
            "abc@ingest.mntogether.org"
        );
    }
}
