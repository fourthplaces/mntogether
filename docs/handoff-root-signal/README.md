# Root Signal → Root Editorial Handoff Package

This folder specifies the ingest integration between Root Signal (the scouting + extraction pipeline) and Root Editorial (the curated CMS). It is the current contract between the two systems. Every consumer-visible behaviour, every field, every constraint either lives here or in a doc referenced from here.

The integration it describes is the critical next step in Root Suite. Signal has no channel to deliver its work product today; Editorial runs on seed data. These documents close that gap.

---

## What's in this folder

| File | What it is |
|---|---|
| [`ROOT_SIGNAL_API_REQUEST.md`](ROOT_SIGNAL_API_REQUEST.md) | The full spec — envelope, field contract, validation, auth, mapping guide, worked examples. Read this first and read all of it. |
| [`TAXONOMY_EXPANSION_BRIEF.md`](TAXONOMY_EXPANSION_BRIEF.md) | The argument for expanding Root Signal's six signal types to cover Profile, LocalBusiness, Opportunity, and Job. Scouting strategies per new type, exclusion criteria, field shapes. Companion to §15 of the request doc. |
| [`TAG_VOCABULARY.md`](TAG_VOCABULARY.md) | Controlled vocabulary for `tags.topic[]`, `tags.service_area[]`, and `tags.safety[]`. |
| `README.md` | This file. |

---

## Companion docs in the Editorial repo

These are referenced throughout the spec and live alongside this folder under `docs/`. Read them when the spec points to them; they are part of the full picture.

| Doc | Purpose |
|---|---|
| [`docs/architecture/ROOT_SIGNAL_DATA_CONTRACT.md`](../architecture/ROOT_SIGNAL_DATA_CONTRACT.md) | The authoritative on-the-wire contract. `ROOT_SIGNAL_API_REQUEST.md` wraps and extends it. Diff the two if you want the source-of-truth-iest version of the envelope. |
| [`docs/architecture/POST_TYPE_SYSTEM.md`](../architecture/POST_TYPE_SYSTEM.md) | Design rationale for Editorial's 9-type post taxonomy. Context for the Signal→Editorial mapping table in the request doc §15.1. |
| [`docs/architecture/SIGNAL_INBOX.md`](../architecture/SIGNAL_INBOX.md) | Admin-UI design for Editorial's queue of `in_review` posts — where soft-fail submissions (`extraction_confidence < 60`, `duplicate_of_id` set, etc.) land for editor clearance. |
| [`docs/architecture/DATABASE_SCHEMA.md`](../architecture/DATABASE_SCHEMA.md) | Editorial schema reference. |
| [`docs/architecture/PII_SCRUBBING.md`](../architecture/PII_SCRUBBING.md) | Editorial's PII policy. Relevant if scraped bodies may surface personal information. |
| [`data/tags.json`](../../data/tags.json) | Raw tag vocabulary JSON. `TAG_VOCABULARY.md` in this folder is the readable form. |
| [`data/audit-seed.mjs`](../../data/audit-seed.mjs) | The validation pass that checks Editorial's seed corpus against the contract. A working reference implementation of the validation rules the ingest endpoint applies. |

---

## Operational exchanges at kickoff

A few pieces of operational context are exchanged directly between the two teams at integration kickoff, not documented here:

- **API credentials.** Editorial issues Bearer tokens per environment (`rsk_dev_…`, `rsk_test_…`, `rsk_live_…`) over a secure channel. Key format, rotation semantics, and the `ServiceClient` auth extractor are specified in the request doc §14.1.
- **Production and staging URLs.** Issued alongside the credentials.
- **Named contacts + on-call rotation.** Editorial owns ingest-endpoint availability; Root Signal owns content quality and volume profile. Escalation paths exchanged at kickoff.
- **Signal detail URL pattern.** The base URL where `[signal:UUID]` citation tokens resolve on Root Signal's public site (e.g., `https://signal.example.com/signals/<uuid>`). Editorial's citation renderer takes this as configuration.
