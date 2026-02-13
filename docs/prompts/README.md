# LLM Prompts

All LLM prompts used in the codebase, extracted for reference and review.

## Extraction Pipeline (`packages/extraction/`)

| Prompt | Purpose |
|--------|---------|
| [summarize.md](extraction-pipeline/summarize.md) | Summarize a webpage for information retrieval |
| [expand-query.md](extraction-pipeline/expand-query.md) | Expand search queries with related terms |
| [classify-query.md](extraction-pipeline/classify-query.md) | Classify search query intent |
| [partition.md](extraction-pipeline/partition.md) | Partition pages into distinct items |
| [extract.md](extraction-pipeline/extract.md) | Evidence-grounded extraction (COLLECTION strategy) |
| [extract-single.md](extraction-pipeline/extract-single.md) | Single-answer extraction (SINGULAR strategy) |
| [extract-narrative.md](extraction-pipeline/extract-narrative.md) | Narrative summary extraction (NARRATIVE strategy) |

## Crawling Domain (`packages/server/.../crawling/`)

Three-pass post extraction pipeline:

| Prompt | Purpose |
|--------|---------|
| [narrative-extraction.md](crawling/narrative-extraction.md) | Pass 1: Extract narrative posts from website content |
| [dedupe.md](crawling/dedupe.md) | Pass 2: Deduplicate and merge posts across batches |
| [investigation.md](crawling/investigation.md) | Pass 3: Agentic investigation for contact info |
| [structured-extraction.md](crawling/structured-extraction.md) | Extract structured fields from investigation findings |

## Posts Domain (`packages/server/.../posts/`)

| Prompt | Purpose |
|--------|---------|
| [extract-posts-raw.md](posts/extract-posts-raw.md) | Extract listings from a single page |
| [extract-posts-batch.md](posts/extract-posts-batch.md) | Extract listings from multiple pages |
| [generate-summary.md](posts/generate-summary.md) | Generate concise TLDR from description |
| [generate-outreach.md](posts/generate-outreach.md) | Generate volunteer outreach email |

## Post Sync (`packages/server/.../posts/`)

| Prompt | Purpose |
|--------|---------|
| [sync-system.md](sync/sync-system.md) | Sync fresh extractions against existing DB posts |
| [sync-schema.md](sync/sync-schema.md) | Schema hint for sync output format |

## Deduplication (`packages/server/.../posts/`)

| Prompt | Purpose |
|--------|---------|
| [dedup-system.md](dedup/dedup-system.md) | Identify duplicate posts within a website |
| [dedup-pending.md](dedup/dedup-pending.md) | Identify duplicates among draft posts only |
| [dedup-match-pending-active.md](dedup/dedup-match-pending-active.md) | Match draft posts against published posts |
| [dedup-schema.md](dedup/dedup-schema.md) | Schema hint for dedup output |
| [dedup-match-schema.md](dedup/dedup-match-schema.md) | Schema hint for pending-active matching |
| [dedup-merge-reason.md](dedup/dedup-merge-reason.md) | Generate user-friendly merge explanation |

## PII Detection (`packages/server/.../common/pii/`)

| Prompt | Purpose |
|--------|---------|
| [pii-detection.md](pii/pii-detection.md) | Context-aware PII detection |
