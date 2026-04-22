//! Magic-bytes format detection for ingested images.
//!
//! The Content-Type header is a hint, not a promise — a hostile or
//! careless upstream can claim `image/jpeg` on an HTML exploit blob,
//! or `image/svg+xml` on a WebP, or nothing at all. Every ingested
//! byte stream gets classified here against the leading bytes of the
//! file before any decode step runs.
//!
//! Supports the four formats in handoff §9.1: JPEG, PNG, WebP, AVIF.
//! Everything else hard-fails.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Avif,
}

impl ImageFormat {
    /// MIME type we'll report to downstream code. Does not necessarily
    /// match the `Content-Type` the upstream sent.
    pub fn mime(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Webp => "image/webp",
            ImageFormat::Avif => "image/avif",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidateError {
    #[error("body is too short to classify ({0} bytes)")]
    TooShort(usize),
    #[error("body does not match any allowed image format")]
    UnrecognisedFormat,
}

/// Inspect the leading bytes of `body` and return the format, or an
/// error if it matches none of {JPEG, PNG, WebP, AVIF}.
pub fn detect_format(body: &[u8]) -> Result<ImageFormat, ValidateError> {
    if body.len() < 12 {
        return Err(ValidateError::TooShort(body.len()));
    }

    // JPEG: FF D8 FF (SOI marker + first app segment tag).
    if body[0..3] == [0xFF, 0xD8, 0xFF] {
        return Ok(ImageFormat::Jpeg);
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A (PNG signature).
    if body[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Ok(ImageFormat::Png);
    }

    // WebP: "RIFF" <4-byte size> "WEBP".
    if &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP" {
        return Ok(ImageFormat::Webp);
    }

    // AVIF: ISO Base Media File Format container. Bytes 4..8 are the
    // box type "ftyp"; bytes 8..12 carry the major brand, one of
    // "avif" (still) or "avis" (image sequence) for AVIF.
    if &body[4..8] == b"ftyp" {
        let brand = &body[8..12];
        if brand == b"avif" || brand == b"avis" {
            return Ok(ImageFormat::Avif);
        }
    }

    Err(ValidateError::UnrecognisedFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal fixtures: just enough leading bytes for detect_format to
    // classify. Not valid decodable images — that's the normaliser's
    // problem, not ours.
    const JPEG_HEAD: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 0, 0, 0, 0];
    const PNG_HEAD: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0,
    ];
    const WEBP_HEAD: &[u8] = b"RIFF\0\0\0\0WEBPVP8L";
    const AVIF_HEAD: &[u8] = b"\0\0\0\x20ftypavif\0\0\0\0";
    const AVIS_HEAD: &[u8] = b"\0\0\0\x20ftypavis\0\0\0\0";

    #[test]
    fn detects_jpeg() {
        assert_eq!(detect_format(JPEG_HEAD).unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn detects_png() {
        assert_eq!(detect_format(PNG_HEAD).unwrap(), ImageFormat::Png);
    }

    #[test]
    fn detects_webp() {
        assert_eq!(detect_format(WEBP_HEAD).unwrap(), ImageFormat::Webp);
    }

    #[test]
    fn detects_avif_still_and_sequence() {
        assert_eq!(detect_format(AVIF_HEAD).unwrap(), ImageFormat::Avif);
        assert_eq!(detect_format(AVIS_HEAD).unwrap(), ImageFormat::Avif);
    }

    #[test]
    fn rejects_svg() {
        // A Content-Type-claimed-image that's actually SVG/HTML must
        // fail: no magic bytes match.
        let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg">"#;
        assert!(matches!(detect_format(svg), Err(ValidateError::UnrecognisedFormat)));
    }

    #[test]
    fn rejects_gif() {
        // Root Editorial doesn't host animation. GIFs are rejected
        // here — Signal shouldn't submit them in the first place.
        let gif = b"GIF89a\x01\x00\x01\x00\x00\xFF";
        assert!(matches!(detect_format(gif), Err(ValidateError::UnrecognisedFormat)));
    }

    #[test]
    fn rejects_html() {
        let html = b"<!DOCTYPE html><html><body></body></html>";
        assert!(matches!(detect_format(html), Err(ValidateError::UnrecognisedFormat)));
    }

    #[test]
    fn rejects_non_avif_ftyp() {
        // ISO box with a non-AVIF brand (e.g. mp4) — ftyp alone is not
        // enough; the brand must be avif/avis.
        let mp4 = b"\0\0\0\x20ftypmp42\0\0\0\0";
        assert!(matches!(detect_format(mp4), Err(ValidateError::UnrecognisedFormat)));
    }

    #[test]
    fn rejects_short_body() {
        assert!(matches!(detect_format(&[0xFF, 0xD8]), Err(ValidateError::TooShort(2))));
    }

    #[test]
    fn riff_without_webp_fourcc_rejected() {
        // WAVE file (RIFF but not WebP).
        let wav = b"RIFF\0\0\0\0WAVEfmt ";
        assert!(matches!(detect_format(wav), Err(ValidateError::UnrecognisedFormat)));
    }
}
