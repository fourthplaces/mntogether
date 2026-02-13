# Database Schema Architecture

> Last updated from migrations through `000171`. This is the canonical reference for the current database schema.

## Overview

MN Together uses PostgreSQL with the following extensions:

- **uuid-ossp** — UUID generation
- **pgvector** — Vector similarity search (1536-dimension embeddings for OpenAI)
- **pg_trgm** — Trigram text similarity

The schema spans **~90 tables across 23 business domains**, connected through a combination of foreign keys, polymorphic joins, and class table inheritance.

---

## Core Entities

### members

Privacy-first user representation. No PII stored — only an anonymous push token.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| expo_push_token | TEXT UNIQUE | Anonymous identifier for notifications |
| searchable_text | TEXT | Free-text source of truth for skills/capabilities |
| latitude, longitude | FLOAT | Coarse location for matching |
| location_name | TEXT | Display name ("Minneapolis, MN") |
| active | BOOLEAN | |
| notification_count_this_week | INT | Rate limiting |
| paused_until | TIMESTAMPTZ | Temporary pause |
| created_at | TIMESTAMPTZ | |

### identifiers

Phone-based authentication. Links hashed phone numbers to members.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| member_id | UUID FK → members | |
| phone_hash | VARCHAR(64) UNIQUE | Hashed phone number — never stored in plaintext |
| is_admin | BOOLEAN | |
| created_at, updated_at | TIMESTAMPTZ | |

### organizations

The central entity representing any organization in the directory.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| name | TEXT NOT NULL | |
| description | TEXT | All narrative content lives here |
| submitter_type | TEXT | 'admin', 'public_user', 'system' |
| status | TEXT | 'pending_review', 'approved', 'rejected', 'suspended' |
| submitted_by | UUID FK → members | |
| reviewed_by | UUID FK → members | |
| reviewed_at | TIMESTAMPTZ | |
| rejection_reason | TEXT | |
| last_extracted_at | TIMESTAMPTZ | When data was last extracted from source |
| created_at, updated_at | TIMESTAMPTZ | |

### posts

The primary user-facing entity — temporal announcements for services, opportunities, and businesses. Renamed from `listings` at migration 114.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| title | TEXT | |
| description | TEXT | |
| description_markdown | TEXT | Markdown version |
| summary | TEXT | Short summary |
| post_type | TEXT | 'service', 'opportunity', 'business' |
| category | TEXT | Primary filter: food, housing, legal, healthcare, etc. |
| capacity_status | TEXT | 'accepting', 'paused', 'at_capacity' |
| urgency | TEXT | 'low', 'medium', 'high', 'urgent' |
| status | TEXT | 'pending_approval', 'active', 'filled', 'rejected', 'expired' |
| location | TEXT | |
| latitude, longitude | FLOAT | |
| source_language | TEXT | Supports multi-language content |
| submission_type | TEXT | 'scraped', 'admin', 'org_submitted', 'agent', 'revision' |
| submitted_by_id | UUID FK → members | Both humans and AI agents are members |
| source_url | TEXT | Original page URL for traceability |
| revision_of_post_id | UUID FK → posts | Self-referential for draft revisions |
| translation_of_id | UUID FK → posts | Self-referential for translations |
| duplicate_of_id | UUID FK → posts | Points to canonical post after dedup |
| comments_container_id | UUID FK → containers | For post comments |
| embedding | vector(1536) | Semantic search |
| relevance_score | REAL | Human review triage score |
| relevance_breakdown | TEXT | |
| scored_at | TIMESTAMPTZ | |
| published_at | TIMESTAMPTZ | |
| deleted_at | TIMESTAMPTZ | Soft delete |
| deleted_reason | TEXT | |
| created_at, updated_at | TIMESTAMPTZ | |

**Key indexes**: status, category, latitude/longitude, embedding (HNSW), published_at

---

## Source System (Class Table Inheritance)

Sources use a parent table with specialized child tables connected via 1:1 unique foreign keys. Renamed from `domains` at migration 131.

### sources (parent)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| source_type | TEXT | 'website', 'instagram', 'facebook', 'tiktok' |
| url | TEXT | |
| organization_id | UUID FK → organizations | |
| status | TEXT | 'pending_review', 'approved', 'rejected', 'suspended' |
| active | BOOLEAN | |
| scrape_frequency_hours | INT | |
| last_scraped_at | TIMESTAMPTZ | |
| submitted_by | UUID | |
| submitter_type | TEXT | |
| submission_context | TEXT | |
| reviewed_by, reviewed_at, rejection_reason | | Approval workflow fields |
| created_at, updated_at | TIMESTAMPTZ | |

### website_sources (child, 1:1)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| source_id | UUID UNIQUE FK → sources | |
| domain | TEXT UNIQUE | |
| max_crawl_depth | INT | |
| crawl_rate_limit_seconds | INT | |
| is_trusted | BOOLEAN | |

### social_sources (child, 1:1)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| source_id | UUID UNIQUE FK → sources | |
| source_type | TEXT | Denormalized for UNIQUE constraint |
| handle | TEXT | |

**Constraint**: UNIQUE(source_type, handle)

---

## Crawling & Extraction Pipeline

This is the data pipeline from raw HTML to structured, actionable data:

```
website_sources
    → website_snapshots (tracking)
        → page_snapshots (immutable content)
            → detections (does page contain relevant info?)
            → extractions (structured data pulled from page)
                → field_provenance (where in the page each field came from)
                → relationships (graph edges between extractions)
```

### page_snapshots

Immutable snapshots of crawled pages.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| url | TEXT | |
| content_hash | BYTEA | |
| html | TEXT | Full HTML |
| markdown | TEXT | Converted markdown |
| fetched_via | TEXT | Fetch method |
| metadata | JSONB | Extensible (acceptable JSONB — truly unstructured) |
| crawled_at | TIMESTAMPTZ | |

### website_snapshots

Tracks which pages belong to which website and their scrape status.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| website_id | UUID FK → website_sources.source_id | |
| page_url | TEXT | |
| page_snapshot_id | UUID FK → page_snapshots | |
| scrape_status | TEXT | 'pending', 'scraped', 'failed' |
| last_scraped_at, submitted_at | TIMESTAMPTZ | |

### detections

AI/heuristic signals that a page contains relevant content.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| page_snapshot_id | UUID FK → page_snapshots | |
| kind | TEXT | Detection type |
| confidence_overall, confidence_heuristic, confidence_ai | REAL | |
| origin, evidence | JSONB | |
| detected_at | TIMESTAMPTZ | |

### extractions

Structured data extracted from pages.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| fingerprint | BYTEA | |
| page_snapshot_id | UUID FK → page_snapshots | |
| schema_id | UUID FK → schemas | |
| schema_version | INT | |
| data | JSONB | Extracted structured data |
| confidence_overall, confidence_heuristic, confidence_ai | REAL | |
| origin, evidence | JSONB | |
| extracted_at | TIMESTAMPTZ | |

**Constraint**: UNIQUE(fingerprint, schema_id, schema_version)

### field_provenance

Traces each extracted field back to its source location in the page.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| extraction_id | UUID FK → extractions | |
| field_path | TEXT | JSON path to the field |
| source_location | TEXT | Location in page |
| extraction_method | TEXT | |

### relationships

First-class graph edges between extractions.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| from_extraction_id | UUID FK → extractions | |
| to_extraction_id | UUID FK → extractions | |
| kind | TEXT | |
| confidence_overall, confidence_heuristic, confidence_ai | REAL | |
| origin, metadata | JSONB | |
| created_at | TIMESTAMPTZ | |

**Constraint**: UNIQUE(from_extraction_id, to_extraction_id, kind)

---

## Post Source Tracking

### post_sources

Maps posts to their origin sources, enabling traceability and deduplication.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| post_id | UUID FK → posts | |
| source_type | TEXT | 'website', 'instagram', 'facebook' |
| source_id | UUID FK → sources | |
| content_hash | TEXT | For dedup detection |
| source_url | TEXT | |
| first_seen_at, last_seen_at | TIMESTAMPTZ | Lifecycle tracking |
| disappeared_at | TIMESTAMPTZ | When content was no longer found at source |
| created_at, updated_at | TIMESTAMPTZ | |

**Constraint**: UNIQUE(post_id, source_type, source_id)

### post_contacts

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| post_id | UUID FK → posts | |
| contact_type | TEXT | 'phone', 'email', 'website', 'address' |
| contact_value | TEXT | |
| contact_label | TEXT | |
| display_order | INT | |

---

## Tagging System

Universal polymorphic tagging across all entity types.

### tags

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| kind | TEXT | Category of tag |
| value | TEXT | Specific tag value |
| display_name | TEXT | Human-readable label |
| parent_tag_id | UUID FK → tags | Self-referential hierarchy |
| color | TEXT | Hex color for UI |
| description | TEXT | |
| emoji | TEXT | |
| created_at | TIMESTAMPTZ | |

**Constraint**: UNIQUE(kind, value)

**Pre-populated tag kinds**:

| Kind | Example Values |
|------|---------------|
| community_served | somali, ethiopian, latino, hmong, karen, oromo |
| service_area | minneapolis, st_paul, bloomington, brooklyn_park, statewide |
| population | seniors, youth, families, veterans, lgbtq |
| org_leadership | community_led, immigrant_founded, bipoc_led |
| verification_source | admin_verified, community_vouched, self_reported |
| safety | no_id_required, no_authority_contact, ice_safe, confidential, anonymous_ok |

### taggables (polymorphic join)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| tag_id | UUID FK → tags | |
| taggable_type | TEXT | 'post', 'organization', 'provider', 'container', 'website', etc. |
| taggable_id | UUID | Entity ID |
| added_at | TIMESTAMPTZ | |

**Constraint**: UNIQUE(tag_id, taggable_type, taggable_id)

---

## Contacts System (Polymorphic)

### contacts

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| contactable_type | TEXT | 'organization', 'listing', 'provider', 'resource' |
| contactable_id | UUID | |
| contact_type | TEXT | 'phone', 'email', 'website', 'address', 'booking_url', 'social' |
| contact_value | TEXT | |
| contact_label | TEXT | 'Office', 'Mobile', 'LinkedIn' |
| is_public | BOOLEAN | |
| display_order | INT | |
| created_at | TIMESTAMPTZ | |

**Constraint**: UNIQUE(contactable_type, contactable_id, contact_type, contact_value)

---

## Providers Directory

Professional services directory (coaches, therapists, counselors).

### providers

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| name | TEXT | |
| bio, why_statement, headline | TEXT | |
| profile_image_url | TEXT | |
| member_id | UUID FK → members | Optional link to member |
| source_id | UUID FK → sources | Optional link to source |
| location | TEXT | |
| latitude, longitude | FLOAT | |
| service_radius_km | FLOAT | |
| offers_in_person, offers_remote | BOOLEAN | |
| accepting_clients | BOOLEAN | |
| status | TEXT | Approval workflow |
| submitted_by, reviewed_by, reviewed_at, rejection_reason | | |
| embedding | vector(1536) | Semantic matching |
| created_at, updated_at | TIMESTAMPTZ | |

---

## Chat & Messaging

### containers

Generic message containers used for AI chats, post comments, etc.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| language | TEXT FK → active_languages | |
| created_at, last_activity_at | TIMESTAMPTZ | |

### messages

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| container_id | UUID FK → containers | |
| role | TEXT | 'user', 'assistant', 'comment' |
| content | TEXT | |
| author_id | UUID FK → members | Nullable for anonymous |
| moderation_status | TEXT | 'approved', 'pending', 'flagged', 'removed' |
| parent_message_id | UUID FK → messages | Threading |
| sequence_number | INT | |
| created_at, updated_at, edited_at | TIMESTAMPTZ | |

### referral_documents

Generated referral guides. Public, no auth required — uses secret edit tokens.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| container_id | UUID FK → containers | |
| source_language | TEXT FK → active_languages | |
| content | TEXT | Markdown + JSX components |
| slug | TEXT UNIQUE | Human-readable URL |
| title | TEXT | |
| status | TEXT | 'draft', 'published', 'archived' |
| edit_token | TEXT UNIQUE | Secret token for editing (no auth) |
| view_count | INT | |
| last_viewed_at | TIMESTAMPTZ | |
| created_at, updated_at | TIMESTAMPTZ | |

---

## Multi-Language Support

### active_languages

| Column | Type | Notes |
|--------|------|-------|
| language_code | TEXT PK | ISO 639-1 (en, es, so) |
| language_name | TEXT | |
| native_name | TEXT | |
| enabled | BOOLEAN | |
| added_at | TIMESTAMPTZ | |

**Seeded**: English (en), Spanish (es), Somali (so)

### listing_translations

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| listing_id | UUID FK → posts | |
| language_code | TEXT FK → active_languages | |
| title, description, tldr | TEXT | |
| translated_at | TIMESTAMPTZ | |
| translation_model | TEXT | e.g. 'gpt-4o' |

**Constraint**: UNIQUE(listing_id, language_code)

---

## Location System

### locations

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| latitude, longitude | NUMERIC | |
| city, state, country, zip_code | TEXT | |
| display_name, address | TEXT | |

### zip_codes

Pre-computed zip code reference data.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| zip_code | TEXT | |
| latitude, longitude | NUMERIC | |
| city, state, county | TEXT | |

### locationables (polymorphic join)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| locationable_type | TEXT | 'post', 'organization', 'provider' |
| locationable_id | UUID | |
| location_id | UUID FK → locations | |
| added_at | TIMESTAMPTZ | |

### heat_map_points

Aggregated location data for geographic visualization.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| location_id | UUID FK → locations | |
| entity_type | TEXT | |
| entity_count | INT | |
| updated_at | TIMESTAMPTZ | |

---

## Discovery Engine

### discovery_queries

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| query_text | TEXT | |
| category | TEXT | |
| is_active | BOOLEAN | |
| created_by | UUID FK → members | |
| created_at, updated_at | TIMESTAMPTZ | |

### discovery_filter_rules

Plain-text filter rules. NULL query_id means global rule.

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| query_id | UUID FK → discovery_queries | NULL = global |
| rule_text | TEXT | |
| sort_order | INT | |
| is_active | BOOLEAN | |
| created_by | UUID FK → members | |

### discovery_runs

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| queries_executed, total_results, websites_created, websites_filtered | INT | |
| started_at, completed_at | TIMESTAMPTZ | |
| trigger_type | TEXT | |

### discovery_run_results

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| run_id | UUID FK → discovery_runs | |
| query_id | UUID FK → discovery_queries | |
| domain, url, title, snippet | TEXT | |
| relevance_score | DOUBLE PRECISION | |
| filter_result | TEXT | 'pending', 'accepted', 'rejected' |
| filter_reason | TEXT | |
| website_id | UUID FK → sources | |
| discovered_at | TIMESTAMPTZ | |

---

## Synchronization & Deduplication

### sync_proposals

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| proposal_type | TEXT | 'merge', 'deduplicate' |
| primary_entity_id, secondary_entity_id | UUID | |
| status | TEXT | 'pending', 'approved', 'rejected' |
| similarity_score | DECIMAL | |
| reason | TEXT | |
| created_by | UUID FK → members | |
| created_at, updated_at | TIMESTAMPTZ | |

### sync_batches

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| batch_type | TEXT | |
| status | TEXT | 'pending', 'processing', 'completed', 'failed' |
| started_at, completed_at | TIMESTAMPTZ | |

---

## Assessments & Research

### website_assessments

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| website_id | UUID FK → website_sources.source_id | |
| assessment_type | TEXT | |
| findings | JSONB | |
| confidence_score | DECIMAL | |
| created_at, updated_at | TIMESTAMPTZ | |

### website_research

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| website_id | UUID FK → website_sources.source_id | |
| research_type | TEXT | |
| findings | TEXT | |
| embedding | vector(1536) | |

---

## Reporting & Moderation

### post_reports

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| post_id | UUID FK → posts | |
| reported_by | UUID FK → members | Nullable for anonymous reports |
| reporter_email | TEXT | |
| reason | TEXT | |
| category | TEXT | 'inappropriate_content', 'spam', 'misleading_information', 'duplicate', 'outdated', 'offensive' |
| status | TEXT | 'pending', 'resolved', 'dismissed' |
| resolved_by | UUID | |
| resolved_at | TIMESTAMPTZ | |
| resolution_notes | TEXT | |
| action_taken | TEXT | 'listing_deleted', 'listing_rejected', 'listing_updated', 'no_action' |
| created_at, updated_at | TIMESTAMPTZ | |

---

## Notes & Caching

### notes

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| title, content | TEXT | |
| severity | TEXT | 'info', 'warning', 'urgent' |
| cta_text | TEXT | Call-to-action |
| embedding | vector(1536) | |

### memo_cache

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| cache_key | TEXT UNIQUE | |
| cached_value | TEXT | |
| expires_at | TIMESTAMPTZ | |

---

## Scheduling

### schedules (polymorphic)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| schedulable_type, schedulable_id | TEXT, UUID | Polymorphic |
| day_of_week | INT | |
| open_time, close_time | TIME | |
| is_open | BOOLEAN | |

---

## Design Patterns

### 1. Privacy-First Members

Members store zero PII. The `expo_push_token` is the anonymous identifier. Phone numbers are hashed in the `identifiers` table and never stored in plaintext. Location is coarse (city-level).

### 2. Class Table Inheritance (Sources)

The `sources` parent table holds shared fields (status, scrape config, approval workflow). Specialized child tables (`website_sources`, `social_sources`) extend it via 1:1 unique foreign keys. Query the parent for cross-type operations, join the child for type-specific data.

```
sources (parent)
├── website_sources (1:1) — domain, crawl config, trust
└── social_sources (1:1) — handle, platform
```

### 3. Temporal Announcements (Posts)

Posts are time-bounded announcements created from underlying organization/source state. One organization can produce many posts over time. Posts can be revised (`revision_of_post_id`), translated (`translation_of_id`), or deduplicated (`duplicate_of_id`) via self-referential foreign keys.

### 4. Polymorphic Joins

Four tables use the `(type, id)` polymorphic pattern to connect to multiple entity types:

| Table | Type Column | Supported Types |
|-------|-------------|----------------|
| taggables | taggable_type | post, organization, provider, container, website, resource |
| contacts | contactable_type | organization, listing, provider, resource |
| locationables | locationable_type | post, organization, provider |
| schedules | schedulable_type | post, organization, provider |

### 5. Approval Workflows

Multiple entity types go through a review pipeline before activation:

```
pending_review → approved
               → rejected (with reason)
               → suspended
```

Applied to: **organizations**, **sources**, **providers**, **posts**

Each stores: `status`, `submitted_by`, `submitter_type`, `reviewed_by`, `reviewed_at`, `rejection_reason`

### 6. Source Traceability

Complete lineage from posts back to crawled pages:

```
post → post_sources → source → website_sources
                             → page_snapshots → extractions
```

Content hashes in `post_sources` enable dedup detection across crawl runs. `first_seen_at`, `last_seen_at`, and `disappeared_at` track the lifecycle of source content.

### 7. Vector Search

Three tables carry `vector(1536)` embeddings indexed with HNSW:

- **posts.embedding** — semantic search for services/opportunities
- **providers.embedding** — semantic matching for professional services
- **website_research.embedding** — research similarity

---

## Entity Relationship Diagram

```
┌──────────────┐       ┌──────────────────┐       ┌─────────────────┐
│   members    │──1:N──│   identifiers    │       │ active_languages│
│              │       │  (phone auth)    │       └────────┬────────┘
│ expo_push_   │       └──────────────────┘                │
│ token, loc   │                                           │
└──────┬───────┘                                           │
       │                                                   │
       │ submitted_by                                      │
       ▼                                                   │
┌──────────────┐       ┌──────────────────┐       ┌────────┴────────┐
│organizations │──1:N──│     sources      │       │  translations   │
│              │       │   (parent)       │       └─────────────────┘
│ name, desc,  │       │                  │
│ status       │       │ source_type,     │
└──────┬───────┘       │ status, url      │
       │               └───────┬──────────┘
       │                  1:1  │  1:1
       │            ┌──────────┼──────────┐
       │            ▼                     ▼
       │   ┌────────────────┐   ┌────────────────┐
       │   │website_sources │   │ social_sources  │
       │   │ domain, crawl  │   │ handle, type    │
       │   └───────┬────────┘   └─────────────────┘
       │           │
       │           │ crawl pipeline
       │           ▼
       │   ┌────────────────┐     ┌──────────────┐
       │   │website_snapshots│────│page_snapshots │
       │   └────────────────┘     │ html, markdown│
       │                          └───────┬───────┘
       │                             ┌────┴────┐
       │                             ▼         ▼
       │                      ┌───────────┐ ┌───────────┐
       │                      │detections │ │extractions│
       │                      └───────────┘ └─────┬─────┘
       │                                     ┌────┴────┐
       │                                     ▼         ▼
       │                              ┌──────────┐ ┌──────────────┐
       │                              │field_    │ │relationships │
       │                              │provenance│ │(graph edges) │
       │                              └──────────┘ └──────────────┘
       │
       │  org has many posts
       ▼
┌──────────────┐       ┌──────────────────┐
│    posts     │──N:N──│   post_sources   │──── sources
│              │       │ (traceability)   │
│ title, desc, │       └──────────────────┘
│ type, status,│
│ embedding    │       ┌──────────────────┐
│              │──1:N──│  post_contacts   │
└──────┬───────┘       └──────────────────┘
       │
       │ comments_container_id
       ▼
┌──────────────┐       ┌──────────────────┐
│  containers  │──1:N──│    messages      │
│              │       │ role, content    │
│              │       └──────────────────┘
│              │
│              │──1:N──┌──────────────────┐
│              │       │referral_documents│
└──────────────┘       │ slug, edit_token │
                       └──────────────────┘

┌──────────────┐
│    tags      │──N:N via taggables──→ posts, orgs, providers, etc.
│ kind, value  │
└──────────────┘

┌──────────────┐
│  contacts    │──polymorphic──→ orgs, providers, etc.
└──────────────┘

┌──────────────┐
│  locations   │──N:N via locationables──→ posts, orgs, providers
└──────────────┘

┌──────────────┐
│  providers   │  Professional services directory
│ name, bio,   │  FK → members (optional)
│ embedding    │  FK → sources (optional)
└──────────────┘
```

---

## Schema Evolution History

Key renames and refactors across 171 migrations:

| Migration | Change |
|-----------|--------|
| 114 | `listings` → `posts` (renamed table + all FKs) |
| 131 | `domains` → `sources` (unified website + social sources) |
| Various | `volunteers` → `members` |
| 131+ | Class table inheritance for sources (website_sources, social_sources) |
| 140+ | Extraction pipeline (page_snapshots, detections, extractions) |
| 155+ | Discovery engine (queries, filter rules, runs) |
| 160+ | Post source tracking and traceability |

---

## Indexing Strategy

### Hot-Path Indexes

```sql
-- Post filtering (most common queries)
CREATE INDEX idx_posts_status ON posts(status);
CREATE INDEX idx_posts_category ON posts(category);
CREATE INDEX idx_posts_published_at ON posts(published_at);

-- Location search
CREATE INDEX idx_posts_lat_lng ON posts(latitude, longitude);

-- Approval workflows
CREATE INDEX idx_organizations_status ON organizations(status);
CREATE INDEX idx_sources_status ON sources(status);
CREATE INDEX idx_providers_status ON providers(status);

-- Polymorphic lookups
CREATE INDEX idx_taggables_entity ON taggables(taggable_type, taggable_id);
CREATE INDEX idx_contacts_entity ON contacts(contactable_type, contactable_id);

-- Active post sources (exclude disappeared)
CREATE INDEX idx_post_sources_active ON post_sources(post_id) WHERE disappeared_at IS NULL;
```

### Vector Indexes (HNSW)

```sql
CREATE INDEX idx_posts_embedding ON posts USING hnsw (embedding vector_cosine_ops);
CREATE INDEX idx_providers_embedding ON providers USING hnsw (embedding vector_cosine_ops);
```

---

## Rules of Thumb

1. **Fields for queries, relationships, and CTAs.** Everything narrative goes in `description`. Everything categorical goes in `tags`.
2. **No JSONB for structured data.** Use normalized relational tables. JSONB is acceptable only for truly unstructured data (page metadata, external API responses).
3. **All SQL lives in models.** `domains/*/models/` is the only place for database queries.
4. **Never modify existing migrations.** Always create a new migration file. SQLx checksums are immutable.
5. **Use `sqlx::query_as::<_, Type>` function form.** Never the `query_as!` macro.
