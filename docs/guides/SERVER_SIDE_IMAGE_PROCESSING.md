# Server-side image processing (design proposal, not built)

**Status**: proposed, not implemented. Written 2026-04-17.
**Scope**: future work. The current CMS does resize + compression in the
browser via Canvas (`packages/admin-app/lib/image-processing.ts`). This doc
captures what moving that work to the Rust server would look like, and the
conditions that would motivate doing it.

## Why we started client-side

When we built the upload path, we wanted "hands-off" image resizing + JPEG
compression with the smallest possible blast radius:

- Zero new dependencies.
- Zero new infra.
- Zero changes to the presigned-upload flow (browser PUTs directly to
  MinIO; the Rust server doesn't see the bytes).
- EXIF stripping and orientation-correctness come for free via
  `createImageBitmap({ imageOrientation: 'from-image' })`.

Canvas hits "good enough" quality at the sizes that matter for the
broadsheet (1240px on the longest edge, 0.85 JPEG). The cost is
editor-side CPU — a 12MP phone photo takes maybe 500ms to resize on a
modern laptop.

That's the right default for a solo-dev editorial tool with light upload
volume. The conditions below are what would tip us toward server-side.

## When we'd want to move server-side

1. **Inconsistent output across editors.** Canvas quality varies by
   browser + OS. If we ever need reproducible output (e.g. downstream
   tooling that hashes image bytes, or content-addressed caching), the
   server needs to be the canonical processor.

2. **Variants.** If the broadsheet starts wanting multiple sizes per
   image (thumbnail, card, hero, full-bleed), generating them
   client-side means N canvas passes per upload, and each variant's
   bytes have to go over the wire. Server-side can produce variants
   from one uploaded original without touching the editor's machine
   again.

3. **Format migration.** If we ever want to serve WebP/AVIF as the
   primary delivery format with JPEG fallback (better bytes-per-pixel),
   Rust can do that reliably across all uploads. Canvas can't encode
   AVIF in any browser today.

4. **Root Signal ingest.** When the external-URL ingest feature lands
   (see `ROOT_SIGNAL_MEDIA_INGEST.md`), the CMS will be fetching + re-
   encoding images server-side anyway. At that point the client-side
   path becomes the odd one out; consolidating both paths through the
   same Rust pipeline simplifies the mental model and the operational
   story.

5. **Tamper-proof constraints.** Client-side processing trusts the
   browser to apply limits. A sophisticated editor can bypass it and
   upload a 50MB original. Server-side can enforce `max_bytes` +
   `max_dimensions` regardless of client.

6. **Large volume.** If we ever get to a scale where editors are
   uploading hundreds of images a day and their machines become the
   bottleneck, moving the work to a beefy server becomes attractive.

None of these apply today. File this for the day the first one does.

## Crate choices

The Rust ecosystem has one obvious pick and a few supporting actors.

- **`image`** — the standard image library. Decodes JPEG, PNG, WebP,
  GIF, TIFF, BMP; encodes JPEG, PNG, WebP, GIF. Actively maintained,
  pure Rust. All basic transforms (resize, crop, orient) are built in.
  Documentation is solid. No unsafe linking.

- **`kamadak-exif`** — read EXIF orientation + metadata. `image` itself
  doesn't honor EXIF rotation on decode, so we need this to know
  whether to rotate after loading. (Same situation as Canvas needed
  explicit orientation handling — no free lunch.)

- **`webp`** — a small crate that wraps `libwebp`. Needed if we want
  WebP encoding at quality levels; `image`'s built-in WebP encoder is
  lossless-only as of writing. Pulls in a C dependency.

- **`ravif`** or **`libavif-rs`** — AVIF encoding. AVIF hasn't settled
  in Rust yet; both options have rough edges. Not required for an MVP.

- **`oxipng`** — lossless PNG recompression. Useful if we ever keep PNG
  outputs (transparency, UI assets). Not needed for the
  JPEG-everything default.

For our default (1240px, 0.85 JPEG output, strip EXIF), `image` +
`kamadak-exif` is enough. No new C deps.

## Where to hook it in

Two shapes. Pick one:

### Shape A — proxy uploads through the server

- Browser POSTs the raw file to a new `POST /MediaService/upload`
  endpoint on the Rust server.
- Server decodes with `image`, applies resize + re-encode, uploads to
  MinIO itself using the existing S3 adapter, creates the `media` row,
  returns it.
- Replace the 3-step presigned flow entirely.

Pros: one code path, server fully owns the bytes, easy to add
variants / validation / content-hash dedupe.

Cons: every byte goes browser → server → MinIO. More server bandwidth
+ memory pressure on big uploads. Need to stream carefully (see
"Streaming and memory").

### Shape B — keep presigned PUT, add a post-upload processing step

- Browser uploads the original directly to MinIO (existing flow).
- `confirm_upload` triggers: server downloads the just-uploaded object
  from MinIO, processes, uploads the processed version back (same or
  different storage key), updates the `media` row.
- The `media` row's `url` ends up pointing at the processed version
  regardless.

Pros: keeps the fast direct-to-S3 upload path, server only holds the
image in memory briefly during the reprocess step.

Cons: more moving parts. Original object temporarily lives in MinIO
and has to be cleaned up (or kept for audit — a policy call).

**Recommendation**: Shape A is cleaner when/if we do this. The
direct-to-S3 speed advantage is meaningful only for very large files,
and the editor-facing upload volume is never going to be huge. Single
code path wins.

## Streaming and memory

The naive Rust pipeline reads the whole source into a `Vec<u8>`,
decodes to a `DynamicImage` (which is also in memory), resizes (more
memory), encodes (more memory). For a 20MP source, that's easily
50-100MB of allocation. Not a problem for one upload; becomes a
problem if the server does 50 at once.

Mitigations:

- **Size cap at the edge.** `max_bytes: 20MB` enforced by axum's body
  limit + explicit check after read. Reject anything bigger with a
  413.
- **Concurrency cap.** `tokio::sync::Semaphore` around the processing
  step so at most N uploads are decoding/encoding simultaneously.
- **Stream the MinIO upload.** `aws-sdk-s3` supports
  `ByteStream::from_path` for streaming upload, so we don't need to
  hold the re-encoded bytes after they've been written.
- **`image::io::Reader` with limits.** Has per-decoder memory caps we
  can wire up to refuse malicious 50000×50000 images that would OOM
  us. See `Reader::with_guessed_format()` + `no_limits(false)`.

For our scale, this is all overcautious. Documenting it so we know the
answer when it matters.

## Defaults (matching the current client-side defaults)

Same policy as `packages/admin-app/lib/image-processing.ts`:

- Input types we reprocess: `image/jpeg`, `image/png`, `image/heic`,
  `image/heif` (HEIC support via `image` is limited — may need a
  separate decoder or convert client-side as a fallback).
- Pass through unchanged: WebP, AVIF, GIF, SVG, anything non-image.
- Max dimensions: 1240px on the longest edge.
- Output: JPEG at 0.85 quality.
- Never upscale.
- Strip EXIF (orient correctly, then encode without metadata).
- White backdrop when converting PNG → JPEG.

These match what the client does today, so we'd get parity when the
switch happens.

## Implementation sketch (Shape A)

```rust
// In MediaService::upload handler:
let body = req.body.collect_bytes(MAX_UPLOAD_BYTES).await?;
let original_size = body.len() as i64;

let processed = tokio::task::spawn_blocking(move || {
    let img = ImageReader::new(Cursor::new(&body))
        .with_guessed_format()?
        .decode()?;
    let oriented = apply_exif_orientation(img, exif_reader(&body));
    let resized = oriented.resize(MAX_SIZE, MAX_SIZE, FilterType::Lanczos3);
    let mut buf = Vec::new();
    resized.write_to(&mut Cursor::new(&mut buf), ImageOutputFormat::Jpeg(85))?;
    Ok::<_, anyhow::Error>((buf, resized.width(), resized.height()))
}).await??;

let (bytes, width, height) = processed;
let storage_key = format!("media/{yyyy}/{mm}/{uuid}.jpg", ...);
state.deps.storage.put(&storage_key, bytes, "image/jpeg").await?;
let media = Media::create(... width, height, original_size, storage_key, ...).await?;
Ok(Json(media))
```

Decoding + encoding live inside `spawn_blocking` because `image` is
CPU-bound and synchronous. The semaphore mentioned above wraps this
block.

## What changes for callers

Front-end:

- `useMediaUpload` collapses to a single POST instead of the 3-step
  presigned dance.
- Client-side processing becomes a thin fallback: keep it for offline
  or degraded-connection cases, but don't rely on it for bytes
  reaching the server.

Back-end:

- `presigned_upload` + `confirm_upload` endpoints can be removed (or
  kept for Root Signal ingest, which writes media rows without a
  client).
- New `upload` endpoint handles auth, size limits, processing, S3
  write, row creation.
- Existing `update_metadata`, `delete`, `list`, `list_usage` stay the
  same.

Storage:

- No change. Output still lands in the `media` bucket under the same
  key convention.

## Decision

Client-side path is sufficient today. Revisit when any of the "when
we'd move server-side" conditions applies — the most likely trigger is
#4 (Root Signal ingest), because that path has to be server-side
anyway, and at that point sharing a single Rust pipeline is simpler
than maintaining both.
