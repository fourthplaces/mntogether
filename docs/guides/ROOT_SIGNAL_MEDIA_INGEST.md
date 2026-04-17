# Root Signal → Media Ingest (design proposal, not built)

**Status**: proposed, not implemented. Written 2026-04-17.
**Scope**: future work. The current CMS does not download external images. Editors upload images manually via the admin Media Library, and every production image is hosted on our MinIO bucket.

## Motivation

Root Signal (our upstream content pipeline) often knows about a source image for a post — the photo attached to a press release, a community organization's logo, a stock image bundled with an external event listing. Today if Root Signal wants an image to accompany an ingested post, it has two choices:

1. Pass the image URL as a plain string and let the admin render hotlinked images from the source. **Not acceptable** — hotlinked images break when the source page moves or 404s, can be swapped out from under us, can include tracking pixels, and expose our readers to whatever the source host logs.
2. Skip the image. The resulting posts have no visual treatment in broadsheet layouts, and editors are left manually uploading an image every time if they want visuals.

The proposal is a third option: **Root Signal passes a source URL; the CMS downloads, validates, stores, and references the image on its own terms.**

## Proposed flow

When Root Signal ingests a post or widget with a `source_image_url` field in the payload:

1. **Fetch** the URL server-side with a strict timeout (5s) and size cap (5 MB), following redirects up to N hops. Request with a descriptive `User-Agent` so we can be identified if needed.
2. **Validate** it's really an image:
   - Inspect the first few KB — verify magic bytes match JPEG/PNG/WebP/AVIF (don't trust the HTTP `Content-Type` header alone; malicious servers can misrepresent).
   - Reject anything else.
3. **Normalize**:
   - Convert to WebP at reasonable quality (say 85) to standardize storage and strip EXIF metadata that could leak location/camera data.
   - Record original dimensions.
4. **Dedupe**:
   - Compute a perceptual hash (e.g. pHash or a content SHA-256). Check against existing `media.content_hash`. If match, reuse the existing `media.id` instead of re-uploading.
5. **Store**:
   - Upload to MinIO under the same storage path convention as user uploads (e.g. `media/{yyyy}/{mm}/{uuid}.webp`).
   - Insert a `media` row with these extras:
     - `source_url` (TEXT) — original URL we fetched from
     - `source_ingested_at` (TIMESTAMPTZ) — when we pulled it
     - `content_hash` (TEXT) — for dedupe
     - `alt_text` — see "Alt text" below
6. **Link** to the incoming post/widget via the normal `media_id` FK / `media_references` polymorphic table, so the asset appears in the Media Library with proper usage tracking.
7. **Surface** in the admin as a Root Signal–ingested asset (small badge on the library detail panel showing its source URL + ingest date, so editors can trace provenance).

## Design questions (deferred)

- **Alt text generation**: where does it come from? Options — (a) Root Signal generates it as part of the payload; (b) we use a vision model to caption at ingest time; (c) leave it blank and editors fill it. Probably (a) with (c) as fallback — Root Signal has context we don't (surrounding post body, org metadata).
- **Fair use / licensing**: we're hosting third-party images. Root Signal should pass `source_credit` and `source_license` fields alongside the URL. Without explicit license, we don't ingest. TBD policy on `all-rights-reserved` sources (probably don't host at all).
- **Content addressing vs. reupload**: perceptual hashes allow dedupe, but forks of the same image (cropped, recompressed) will hash differently. Is content-SHA256 enough, or do we want fuzzy matching? Probably start with exact-match SHA256 and add fuzzy later.
- **Retry / failure**: if the fetch fails (timeout, 404, size exceeded), the post still ingests with no image. Should we queue for retry? Probably not — if the source is flaky, the ingest is a one-shot best-effort.
- **GIF / animation**: probably strip animation (convert to WebP still frame) unless someone specifically wants animated content on a community newsletter site. Nothing in the product calls for animation today.
- **Abuse surface**: arbitrary URL fetch = SSRF risk. Ingest must refuse `localhost`, private IP ranges, link-local addresses, `file://` schemes. Validate after DNS resolution, not just before. Run the fetch in a network namespace with egress-only rules if we're paranoid.
- **Rate limiting**: how often can Root Signal trigger ingests? If it's unbounded, bad actors upstream could fill our storage. Probably rate-limit per-source-domain and per-hour at the CMS side.
- **Storage cost**: budget for this? If every ingested post brings a 1 MB image, X posts/day = Y MB/day of storage growth. Do the math when scope firms up.

## What's NOT proposed

- Ingesting non-image media (video, PDFs). Out of scope for the product today.
- A general-purpose "paste any URL to import" feature for editors. External URLs are allowed in the admin only as an escape hatch via an "Advanced" disclosure; they don't flow through this ingest pipeline.
- Crawling or scraping — this feature is pull-on-demand based on URLs Root Signal explicitly supplies, not a generic crawler.

## Relationship to existing work

This proposal builds on the media library unification landing in 2026-04 (see `hazy-giggling-wirth.md` plan). The `media_references` polymorphic table + `media_id` FKs introduced there are the anchor points for linking ingested images to posts/widgets. The presigned-upload path stays the user-facing flow; this ingest path is an additional server-side entry point that writes directly to the `media` table without client involvement.

## Decision

Not building today. Flag for product review when Root Signal is ready to send source image URLs reliably and we have a policy on licensing/credit.
