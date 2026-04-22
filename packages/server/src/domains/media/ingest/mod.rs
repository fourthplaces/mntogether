//! Server-side pipeline that turns a Root Signal–submitted
//! `source_image_url` into a locally-hosted `media` row.
//!
//! Pipeline order (each step hard-fails on its own error variant):
//!
//! ```text
//!   ssrf::validate_url
//!     -> fetch::fetch                   (reqwest, 5s / 5 MiB / HTTPS)
//!     -> validate::detect_format        (magic bytes, not Content-Type)
//!     -> normalise::normalise_to_webp   (decode + re-encode, strips EXIF)
//!     -> sha256(normalised bytes)
//!     -> Media::find_by_content_hash    (exact-match dedup)
//!        ├── hit  -> return existing media_id
//!        └── miss -> storage.put_object + Media::create_ingested
//! ```
//!
//! Thread safety: every step is `Send + Sync`; the whole activity
//! holds no mutable state.
//!
//! The activity entry point lives at
//! [`crate::domains::media::activities::ingest_source_image`]; see
//! there for the `&ServerDeps` contract.

pub mod fetch;
pub mod normalise;
pub mod ssrf;
pub mod validate;
