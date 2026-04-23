//! Decode an ingested image, strip embedded metadata, and re-encode
//! to WebP at quality 85 for storage.
//!
//! Why always re-encode:
//!
//!   * **EXIF strip.** `image::DynamicImage` is a pure pixel buffer —
//!     decoding + re-encoding drops every EXIF / XMP / IPTC block
//!     the source carried. That covers the location / camera /
//!     photographer-identity leakage risk for free; we don't need a
//!     separate exif-scrubbing pass.
//!   * **Format normalisation.** Broadsheet rendering ships one MIME
//!     type. WebP at q=85 is the smallest of the four accepted inputs
//!     with no visible quality loss on photo content.
//!   * **Animation strip.** Animated WebP / (future) animated AVIF
//!     degrades to the first frame. Nothing in the product calls for
//!     animation; see `ROOT_SIGNAL_MEDIA_INGEST.md` "GIF / animation".
//!
//! **AVIF decoding is not implemented in this PR.** Enabling
//! `image/avif-native` pulls in `libdav1d` as a system build
//! dependency, which means rebuilding the production Docker image
//! (out of this worktree's scope). AVIF inputs pass the magic-bytes
//! validator but fail here with `NormaliseError::UnsupportedFormat`.
//! TODO: once Worktree 3's Dockerfile refresh lands, switch the
//! feature on and extend `decode_input` to cover `ImageFormat::Avif`.

use std::io::Cursor;

use image::{DynamicImage, ImageReader};

use super::validate::ImageFormat;

pub const WEBP_QUALITY: f32 = 85.0;

#[derive(Debug, thiserror::Error)]
pub enum NormaliseError {
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("encode failed: {0}")]
    Encode(String),
    #[error("format {0:?} is validated but not yet re-encodable")]
    UnsupportedFormat(ImageFormat),
}

#[derive(Debug)]
pub struct Normalised {
    /// Re-encoded WebP body. Safe to write to storage and hash.
    pub webp_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn normalise_to_webp(
    body: &[u8],
    format: ImageFormat,
) -> Result<Normalised, NormaliseError> {
    let img = decode_input(body, format)?;
    let (width, height) = (img.width(), img.height());
    let webp_bytes = encode_webp(&img, WEBP_QUALITY)?;
    Ok(Normalised {
        webp_bytes,
        width,
        height,
    })
}

fn decode_input(
    body: &[u8],
    format: ImageFormat,
) -> Result<DynamicImage, NormaliseError> {
    let image_format = match format {
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Webp => image::ImageFormat::WebP,
        ImageFormat::Avif => return Err(NormaliseError::UnsupportedFormat(format)),
    };
    let reader = ImageReader::with_format(Cursor::new(body), image_format);
    reader
        .decode()
        .map_err(|e| NormaliseError::Decode(e.to_string()))
}

fn encode_webp(img: &DynamicImage, quality: f32) -> Result<Vec<u8>, NormaliseError> {
    // webp::Encoder wraps libwebp-sys; handles alpha correctly via
    // from_image + lossy quality. Lossless would be `encode_lossless`.
    let encoder = webp::Encoder::from_image(img)
        .map_err(|e| NormaliseError::Encode(e.to_string()))?;
    let mem = encoder.encode(quality);
    Ok(mem.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avif_input_returns_unsupported_for_now() {
        // Fixture doesn't matter — decode_input short-circuits on
        // AVIF before touching bytes.
        let err = normalise_to_webp(&[0; 16], ImageFormat::Avif).unwrap_err();
        assert!(matches!(err, NormaliseError::UnsupportedFormat(ImageFormat::Avif)));
    }

    #[test]
    fn round_trips_a_png() {
        // Encode a tiny PNG in memory so we don't need a fixture file.
        let src = image::DynamicImage::ImageRgb8(image::RgbImage::from_fn(
            8,
            4,
            |x, y| image::Rgb([x as u8 * 16, y as u8 * 32, 128]),
        ));
        let mut png_bytes = Vec::new();
        src.write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
            .unwrap();

        let out = normalise_to_webp(&png_bytes, ImageFormat::Png).unwrap();
        assert_eq!(out.width, 8);
        assert_eq!(out.height, 4);
        assert!(!out.webp_bytes.is_empty());
        // Output should now advertise the WebP RIFF magic.
        assert_eq!(&out.webp_bytes[0..4], b"RIFF");
        assert_eq!(&out.webp_bytes[8..12], b"WEBP");
    }

    #[test]
    fn strips_exif_by_round_tripping_through_dynamic_image() {
        // Synthetic JPEG with a minimal APP1/Exif segment followed by
        // a real JPEG body would be the gold-standard fixture, but
        // DynamicImage's decoder drops all ancillary segments by
        // construction (it only surfaces pixels + ICC). As long as
        // we're going through DynamicImage, EXIF cannot round-trip.
        //
        // This test therefore asserts the *contract*: encode a JPEG
        // with known pixel content, pass it through normalise, and
        // verify the output is a fresh RIFF/WebP container (i.e. not
        // a pass-through of the original bytes + segments).
        let src = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            16,
            16,
            image::Rgb([200, 100, 50]),
        ));
        let mut jpeg_bytes = Vec::new();
        src.write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)
            .unwrap();
        // Pre-check: source is a JPEG.
        assert_eq!(&jpeg_bytes[0..3], &[0xFF, 0xD8, 0xFF]);

        let out = normalise_to_webp(&jpeg_bytes, ImageFormat::Jpeg).unwrap();
        // Post-check: output is a fresh WebP, not a pass-through.
        assert_eq!(&out.webp_bytes[0..4], b"RIFF");
        assert_eq!(&out.webp_bytes[8..12], b"WEBP");
        // And it is not the same byte sequence as the input — if the
        // pipeline ever silently became a pass-through, this fires.
        assert_ne!(out.webp_bytes, jpeg_bytes);
    }
}
