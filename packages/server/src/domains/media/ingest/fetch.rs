//! HTTP fetcher for the Root Signal media ingest pipeline.
//!
//! Hard-coded to the limits in the handoff spec (§9.2):
//!
//!   * 5-second total timeout.
//!   * 5 MiB response-body cap — enforced while streaming, not just by
//!     Content-Length (a malicious server can lie about length, or send
//!     chunked without one).
//!   * HTTPS-only; see [`super::ssrf::validate_url`] — this module
//!     assumes the caller has already validated the URL.
//!   * Redirects followed, but only to HTTPS and only to URLs that
//!     themselves pass the SSRF guard (redirect targets are re-checked,
//!     so a 302 from a public host to `http://169.254.169.254/` fails).
//!
//! DNS-time SSRF is handled differently: `reqwest` does the resolution
//! internally, so we cannot inspect the A/AAAA records before it
//! connects. Instead we rely on (a) the literal-IP check in
//! `validate_url`, and (b) `reqwest`'s TLS enforcement plus the
//! redirect validator. A production hardening follow-up should wire a
//! custom resolver that calls [`super::ssrf::validate_resolved_ips`].
//! The `// TODO:` at the bottom of this file marks that work.

use std::time::Duration;

use bytes::BytesMut;
use futures::StreamExt;
use reqwest::redirect::Policy;
use url::Url;

use super::ssrf::{self, SsrfError};

pub const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
pub const MAX_REDIRECTS: usize = 5;
pub const USER_AGENT: &str =
    "RootEditorial-Ingest/1.0 (+https://rootsignal.example.com/contact)";

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("ssrf: {0}")]
    Ssrf(#[from] SsrfError),
    #[error("fetch timed out after {0:?}")]
    Timeout(Duration),
    #[error("upstream returned status {0}")]
    UpstreamStatus(u16),
    #[error("response body exceeds {MAX_BODY_BYTES}-byte cap")]
    BodyTooLarge,
    #[error("http error: {0}")]
    Http(String),
}

pub struct Fetched {
    pub bytes: Vec<u8>,
    /// The Content-Type header as sent by the upstream. Advisory only —
    /// the validator re-checks the body's magic bytes rather than
    /// trusting this.
    pub upstream_content_type: Option<String>,
    /// The final URL after following redirects.
    pub final_url: Url,
}

/// Fetch `url`, enforcing the 5s / 5 MiB / HTTPS constraints. Returns
/// the raw body bytes — no decoding is performed here.
pub async fn fetch(url: Url) -> Result<Fetched, FetchError> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(FETCH_TIMEOUT)
        .redirect(build_redirect_policy())
        // HTTPS is enforced at the URL level (validate_url) and also at
        // the redirect level (build_redirect_policy). reqwest will still
        // happily follow a redirect to http:// if we let it, so the
        // policy explicitly refuses.
        .https_only(true)
        .build()
        .map_err(|e| FetchError::Http(e.to_string()))?;

    let resp = client.get(url).send().await.map_err(map_send_error)?;

    if !resp.status().is_success() {
        return Err(FetchError::UpstreamStatus(resp.status().as_u16()));
    }

    let final_url = resp.url().clone();
    let upstream_content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Enforce the size cap up front when the server is honest about
    // Content-Length. Servers that omit it or lie still get caught by
    // the streaming check below.
    if let Some(len) = resp.content_length() {
        if len as usize > MAX_BODY_BYTES {
            return Err(FetchError::BodyTooLarge);
        }
    }

    let mut buf = BytesMut::with_capacity(64 * 1024);
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| FetchError::Http(e.to_string()))?;
        if buf.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(FetchError::BodyTooLarge);
        }
        buf.extend_from_slice(&chunk);
    }

    Ok(Fetched {
        bytes: buf.to_vec(),
        upstream_content_type,
        final_url,
    })
}

fn build_redirect_policy() -> Policy {
    Policy::custom(|attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        // Re-run the full URL validator on the redirect target. Catches
        // the "public host 302s to http://10.0.0.1/" trick.
        match ssrf::validate_url(attempt.url().as_str()) {
            Ok(_) => attempt.follow(),
            Err(e) => attempt.error(format!("redirect blocked: {e}")),
        }
    })
}

fn map_send_error(e: reqwest::Error) -> FetchError {
    if e.is_timeout() {
        FetchError::Timeout(FETCH_TIMEOUT)
    } else {
        FetchError::Http(e.to_string())
    }
}

// TODO: wire a custom DNS resolver that calls
// `ssrf::validate_resolved_ips` before handing the IP list to
// hyper_util's connector. `reqwest::ClientBuilder::dns_resolver` +
// `hickory-resolver` is the usual path; out of scope for this PR
// because none of the known Root Signal sources resolve to mixed
// public/private answers, but it's the right next hardening step.
