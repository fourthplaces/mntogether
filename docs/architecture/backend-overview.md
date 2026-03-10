# Backend & GraphQL Overview

> Auto-generated 2026-03-10. Covers Rust server, GraphQL layer, database schema, and frontend operations.

---

## Architecture

```
Next.js Admin (3000) / Web (3001)
        |
    GraphQL (shared/graphql)
        |
    Axum HTTP Server (9080)
        |
    Activities (business logic)
        |
    Models (SQL via sqlx)
        |
    PostgreSQL (5432) + pgvector
```

All HTTP endpoints are POST. GraphQL resolvers call the Rust server via `ServerClient`, which auto-converts snake_case responses to camelCase.

---

## Domains (13)

| Domain | Purpose | Tables |
|---|---|---|
| **auth** | Phone OTP + JWT tokens | `identifiers`, `member_sessions` |
| **posts** | Core content (6 types) | `posts`, `post_sources`, `post_locations`, `post_reports`, `post_contacts` |
| **editions** | County-scoped weekly broadsheets | `editions`, `edition_rows`, `edition_slots`, `edition_sections`, `edition_widgets` |
| **organization** | Community orgs with approval workflow | `organizations`, `organization_checklist` |
| **notes** | Admin annotations linked to posts/orgs | `notes`, `noteables` |
| **tag** | Universal taxonomy | `tags`, `taggables`, `tag_kinds` |
| **member** | App users + push notifications | `members` |
| **contacts** | Polymorphic contact info | `contacts` |
| **schedules** | Event/operating hours | `schedules` |
| **locations** | Geographic data | `locations`, `locationables`, `zip_codes`, `counties`, `zip_counties` |
| **media** | S3 uploads | `media_library` |
| **heat_map** | Geographic visualization | `heat_map_points` |
| **memo** | Computation cache | `memo_cache` |

---

## Post Types & Statuses

**Types**: story, notice, exchange, event, spotlight, reference

**Weights**: heavy, medium, light (controls column width in editions)

**Statuses**: draft, active, rejected, expired, archived, filled

**Urgency**: NULL (none), notice, urgent

**Capacity**: accepting, paused, at_capacity

---

## HTTP Endpoints

### Auth (3)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/Auth/send_otp` | None | Send OTP to phone |
| POST | `/Auth/verify_otp` | None | Verify OTP, return JWT |
| POST | `/Auth/logout` | User | Invalidate session |

### Posts (22)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/Posts/list` | Admin | Paginated list with filters |
| POST | `/Posts/admin_create` | Admin | Create post |
| POST | `/Posts/admin_update` | Admin | Update post |
| POST | `/Posts/approve` | Admin | Approve to active |
| POST | `/Posts/reject` | Admin | Reject with reason |
| POST | `/Posts/delete` | Admin | Soft delete |
| POST | `/Posts/submit` | Optional | Public submission |
| POST | `/Posts/search_semantic` | Admin | Vector similarity search |
| POST | `/Posts/get_upcoming_events` | Admin | Future events |
| POST | `/Posts/backfill_embeddings` | Admin | Batch generate embeddings |
| POST | `/Posts/backfill_locations` | Admin | Batch geocode |
| POST | `/Posts/expire_scheduled` | Admin | Batch expire past events |
| POST | `/Posts/track_view` | Optional | Increment view count |
| POST | `/Posts/track_click` | Optional | Increment click count |
| POST | `/Post/{id}/get` | Optional | Get single post |
| POST | `/Post/{id}/get_by_source` | Optional | Lookup by source URL |
| POST | `/Post/{id}/edit_and_approve` | Admin | Update + approve |
| POST | `/Post/{id}/expire` | Admin | Mark expired |
| POST | `/Post/{id}/archive` | Admin | Archive |
| POST | `/Post/{id}/score_relevance` | Admin | AI relevance score |
| POST | `/Post/{id}/add_tag` | Admin | Add tag |
| POST | `/Post/{id}/remove_tag` | Admin | Remove tag |
| POST | `/Post/{id}/update_tags` | Admin | Bulk replace tags |

### Editions (20)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/Editions/list` | Admin | Paginated editions |
| POST | `/Editions/get_county` | Admin | Single county |
| POST | `/Editions/list_counties` | Admin | All counties |
| POST | `/Editions/get_all_counties` | Admin | All counties (full) |
| POST | `/Editions/list_row_templates` | Admin | Layout templates |
| POST | `/Editions/get_row_template` | Admin | Single template |
| POST | `/Editions/batch_generate` | Admin | Bulk auto-generate |
| POST | `/Editions/batch_approve` | Admin | Bulk approve |
| POST | `/Editions/batch_publish` | Admin | Bulk publish |
| POST | `/Edition/{id}/get` | Admin | Full edition with rows |
| POST | `/Edition/{id}/create` | Admin | Create edition |
| POST | `/Edition/{id}/generate` | Admin | Auto-populate slots |
| POST | `/Edition/{id}/review` | Admin | Move to review |
| POST | `/Edition/{id}/approve` | Admin | Approve edition |
| POST | `/Edition/{id}/publish` | Admin | Publish to public |
| POST | `/Edition/{id}/archive` | Admin | Archive |
| POST | `/Edition/{id}/get_layout_preview` | Admin | Preview layout |
| POST | `/Edition/{id}/update_row` | Admin | Update row config |
| POST | `/Edition/{id}/get_slot` | Admin | Get slot detail |
| POST | `/Edition/{id}/get_widget` | Admin | Get widget detail |

### Organizations (14)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/Organizations/public_list` | None | Public directory |
| POST | `/Organizations/public_get` | None | Public org detail |
| POST | `/Organizations/list` | Admin | All orgs |
| POST | `/Organizations/get` | Admin | Single org |
| POST | `/Organizations/create` | Admin | Create org |
| POST | `/Organizations/update` | Admin | Update org |
| POST | `/Organizations/delete` | Admin | Delete org |
| POST | `/Organizations/approve` | Admin | Approve org |
| POST | `/Organizations/reject` | Admin | Reject org |
| POST | `/Organizations/suspend` | Admin | Suspend org |
| POST | `/Organizations/remove_all_posts` | Admin | Unlink all posts |
| POST | `/Organizations/remove_all_notes` | Admin | Unlink all notes |
| POST | `/Organizations/set_status` | Admin | Generic status change |
| POST | `/Organizations/get_checklist` | Admin | Approval checklist |
| POST | `/Organizations/toggle_checklist_item` | Admin | Toggle checklist |

### Notes (10)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/Notes/create` | Admin | Create note |
| POST | `/Notes/get` | Admin | Get note |
| POST | `/Notes/update` | Admin | Update note |
| POST | `/Notes/delete` | Admin | Delete note |
| POST | `/Notes/list` | Admin | Filtered/paginated |
| POST | `/Notes/list_for_entity` | Admin | Notes for a post/org |
| POST | `/Notes/link` | Admin | Link note to entity |
| POST | `/Notes/unlink` | Admin | Unlink note from entity |
| POST | `/Notes/generate_notes` | Admin | (Stub) |
| POST | `/Notes/attach_notes` | Admin | Auto-attach to org posts |

### Tags (8)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/Tags/list_kinds` | Admin | Tag categories |
| POST | `/Tags/create_kind` | Admin | Create category |
| POST | `/Tags/update_kind` | Admin | Update category |
| POST | `/Tags/delete_kind` | Admin | Delete category |
| POST | `/Tags/list_tags` | Admin | Tags (optionally by kind) |
| POST | `/Tags/create_tag` | Admin | Create tag |
| POST | `/Tags/update_tag` | Admin | Update tag |
| POST | `/Tags/delete_tag` | Admin | Delete tag |

### Media (4)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/MediaService/presigned_upload` | Admin | Get S3 upload URL |
| POST | `/MediaService/confirm_upload` | Admin | Register uploaded file |
| POST | `/MediaService/list` | Admin | Browse media library |
| POST | `/MediaService/delete` | Admin | Delete media |

### Members (5)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/Members/list` | Admin | Paginated members |
| POST | `/Members/run_weekly_reset` | Admin | Reset notification counts |
| POST | `/Member/{id}/get` | Optional | Get member profile |
| POST | `/Member/{id}/update_status` | Admin | Activate/deactivate |
| POST | `/RegisterMemberWorkflow/{key}/run` | None | Register new member |

### Heat Map (2)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/HeatMap/compute_snapshot` | None | Recompute heat map |
| POST | `/HeatMap/get_latest` | None | Get current snapshot |

---

## GraphQL Schema

### Queries

#### Public (unauthenticated)

| Query | Returns | Purpose |
|---|---|---|
| `publicPosts(postType, category, limit, offset, zipCode, radiusMiles)` | `PublicPostConnection!` | Filtered public feed |
| `publicFilters` | `PublicFilters!` | Available filter options |
| `publicOrganizations` | `[PublicOrganization!]!` | Public org directory |
| `publicOrganization(id)` | `PublicOrganization` | Single public org |
| `publicBroadsheet(countyId)` | `PublicBroadsheet` | Current published edition |

#### Posts (admin)

| Query | Returns | Purpose |
|---|---|---|
| `post(id)` | `Post` | Single post (uses dataloader) |
| `posts(status, search, postType, ...)` | `PostConnection!` | Paginated admin list |
| `postStats(status?)` | `PostStats!` | Counts by type |

#### Organizations (admin)

| Query | Returns | Purpose |
|---|---|---|
| `organizations` | `[Organization!]!` | All orgs |
| `organization(id)` | `Organization` | Single org |
| `organizationChecklist(id)` | `Checklist!` | Approval tasks |

#### Tags (admin)

| Query | Returns | Purpose |
|---|---|---|
| `tagKinds` | `[TagKind!]!` | Tag categories |
| `tags(kind?)` | `[Tag!]!` | Tags, optionally filtered |

#### Notes (admin)

| Query | Returns | Purpose |
|---|---|---|
| `note(id)` | `Note` | Single note |
| `notes(severity?, isPublic?, limit?, offset?)` | `NoteConnection!` | Paginated notes |
| `entityNotes(noteableType, noteableId)` | `[Note!]!` | Notes for entity |
| `organizationPosts(organizationId, limit?)` | `PostConnection!` | Posts for org |

#### Editions (admin)

| Query | Returns | Purpose |
|---|---|---|
| `countyDashboard` | `[CountyDashboardRow!]!` | County overview with staleness |
| `counties` | `[County!]!` | All counties |
| `county(id)` | `County` | Single county |
| `editions(countyId?, status?, periodStart?, periodEnd?, limit?, offset?)` | `EditionConnection!` | Paginated editions |
| `latestEditions` | `[Edition!]!` | Latest per county |
| `edition(id)` | `Edition` | Full edition with rows/sections |
| `currentEdition(countyId)` | `Edition` | Currently published |
| `editionPreview(editionId)` | `PublicBroadsheet` | Preview unpublished |
| `editionKanbanStats(periodStart, periodEnd)` | `EditionKanbanStats!` | Workflow stats |
| `rowTemplates` | `[RowTemplate!]!` | Layout templates |
| `postTemplates` | `[PostTemplateConfig!]!` | Post display configs |

#### Media (admin)

| Query | Returns | Purpose |
|---|---|---|
| `mediaLibrary(limit?, offset?, contentType?)` | `MediaConnection!` | Browse uploads |
| `presignedUpload(filename, contentType, sizeBytes)` | `PresignedUpload!` | S3 upload URL |

### Mutations

#### Posts

| Mutation | Returns | Purpose |
|---|---|---|
| `createPost(input)` | `Post!` | Create post |
| `updatePost(id, input)` | `Post!` | Update fields |
| `approvePost(id)` | `Post!` | Approve to active |
| `rejectPost(id, reason?)` | `Post!` | Reject |
| `archivePost(id)` | `Post!` | Archive |
| `deletePost(id)` | `Boolean!` | Delete |
| `reactivatePost(id)` | `Post!` | Reactivate |
| `addPostTag(postId, tagKind, tagValue, displayName?)` | `Post!` | Tag post |
| `removePostTag(postId, tagId)` | `Post!` | Untag post |
| `regeneratePost(id)` | `Post!` | AI rewrite |
| `regeneratePostTags(id)` | `Post!` | AI retag |
| `updatePostCapacity(id, capacityStatus)` | `Post!` | Set capacity |
| `batchScorePosts(limit?)` | `BatchScoreResult!` | Bulk relevance scoring |
| `trackPostView(postId)` | `Boolean` | Analytics |
| `trackPostClick(postId)` | `Boolean` | Analytics |

#### Organizations

| Mutation | Returns | Purpose |
|---|---|---|
| `createOrganization(name, description?)` | `Organization!` | Create |
| `updateOrganization(id, name, description?)` | `Organization!` | Update |
| `deleteOrganization(id)` | `Boolean!` | Delete |
| `approveOrganization(id)` | `Organization!` | Approve |
| `rejectOrganization(id, reason)` | `Organization!` | Reject |
| `suspendOrganization(id, reason)` | `Organization!` | Suspend |
| `setOrganizationStatus(id, status, reason?)` | `Organization!` | Generic status |
| `toggleChecklistItem(organizationId, checklistKey, checked)` | `Checklist!` | Checklist toggle |
| `regenerateOrganization(id)` | `RegenerateOrgResult!` | AI regenerate |

#### Tags

| Mutation | Returns | Purpose |
|---|---|---|
| `createTagKind(slug, displayName, ...)` | `TagKind!` | Create category |
| `updateTagKind(id, ...)` | `TagKind!` | Update category |
| `deleteTagKind(id)` | `Boolean!` | Delete category |
| `createTag(kind, value, ...)` | `Tag!` | Create tag |
| `updateTag(id, ...)` | `Tag!` | Update tag |
| `deleteTag(id)` | `Boolean!` | Delete tag |

#### Notes

| Mutation | Returns | Purpose |
|---|---|---|
| `createNote(noteableType, noteableId, content, ...)` | `Note!` | Create |
| `updateNote(id, content, ...)` | `Note!` | Update |
| `deleteNote(id)` | `Boolean!` | Delete |
| `linkNote(noteId, noteableType, noteableId)` | `Note!` | Link to entity |
| `unlinkNote(noteId, noteableType, noteableId)` | `Boolean!` | Unlink from entity |
| `autoAttachNotes(organizationId)` | `AutoAttachNotesResult!` | Semantic auto-link |

#### Editions (25 mutations)

| Mutation | Returns | Purpose |
|---|---|---|
| `createEdition(countyId, periodStart, periodEnd, title?)` | `Edition!` | Create |
| `generateEdition(id)` | `Edition!` | Auto-populate slots |
| `reviewEdition(id)` | `Edition!` | Move to review |
| `approveEdition(id)` | `Edition!` | Approve |
| `publishEdition(id)` | `Edition!` | Publish |
| `archiveEdition(id)` | `Edition!` | Archive |
| `batchGenerateEditions(periodStart, periodEnd)` | `BatchGenerateEditionsResult!` | Bulk generate |
| `batchApproveEditions(ids)` | `BatchEditionsResult!` | Bulk approve |
| `batchPublishEditions(ids)` | `BatchEditionsResult!` | Bulk publish |
| `addEditionRow(editionId, rowTemplateSlug, sortOrder)` | `EditionRow!` | Add row |
| `deleteEditionRow(rowId)` | `Boolean!` | Remove row |
| `updateEditionRow(rowId, rowTemplateSlug?, sortOrder?)` | `EditionRow!` | Update row |
| `reorderEditionRows(editionId, rowIds)` | `[EditionRow!]!` | Reorder rows |
| `addPostToEdition(editionRowId, postId, postTemplate, slotIndex?)` | `EditionSlot!` | Place post in slot |
| `removePostFromEdition(slotId)` | `Boolean!` | Remove post from slot |
| `changeSlotTemplate(slotId, postTemplate)` | `EditionSlot!` | Change display |
| `moveSlot(slotId, targetRowId, slotIndex?)` | `EditionSlot!` | Move between rows |
| `addWidget(editionRowId, widgetType, slotIndex?, config?)` | `EditionWidget!` | Add widget |
| `updateWidget(id, config)` | `EditionWidget!` | Update widget |
| `removeWidget(id)` | `Boolean!` | Remove widget |
| `addSection(editionId, title, subtitle?, topicSlug?, sortOrder)` | `EditionSection!` | Add section |
| `updateSection(id, title?, subtitle?, topicSlug?)` | `EditionSection!` | Update section |
| `reorderSections(editionId, sectionIds)` | `[EditionSection!]!` | Reorder |
| `deleteSection(id)` | `Boolean!` | Delete section |
| `assignRowToSection(rowId, sectionId?)` | `Boolean!` | Assign row |

#### Media

| Mutation | Returns | Purpose |
|---|---|---|
| `confirmUpload(storageKey, publicUrl, filename, ...)` | `Media!` | Register upload |
| `deleteMedia(id)` | `Boolean!` | Delete file |

---

## Key Data Models (Rust)

### Post

```
id: PostId (UUID)
title: String
description: String
description_markdown: Option<String>
summary: Option<String>
post_type: String (story|notice|exchange|event|spotlight|reference)
weight: String (heavy|medium|light)
status: String (draft|active|rejected|expired|archived|filled)
urgency: Option<String> (notice|urgent)
capacity_status: Option<String>
category: Option<String>
location: Option<String>
latitude: Option<f64>
longitude: Option<f64>
zip_code: Option<String>
priority: i32
organization_id: Option<Uuid>
organization_name: Option<String>
submission_type: Option<String> (scraped|admin|org_submitted)
source_url: Option<String>
embedding: Option<Vector> (1536-dim)
view_count: i32
click_count: i32
body_heavy: Option<String>
body_medium: Option<String>
body_light: Option<String>
created_at: DateTime
updated_at: DateTime
published_at: Option<DateTime>
expired_at: Option<DateTime>
```

### Organization

```
id: OrganizationId (UUID)
name: String
description: Option<String>
status: String (pending_review|approved|rejected|suspended)
submitted_by: Option<Uuid>
reviewed_by: Option<Uuid>
rejection_reason: Option<String>
created_at: DateTime
updated_at: DateTime
```

### Note

```
id: NoteId (UUID)
content: String
severity: Option<String>
is_public: bool
created_by: Option<String>
cta_text: Option<String>
source_url: Option<String>
embedding: Option<Vector>
expired_at: Option<DateTime>
created_at: DateTime
updated_at: DateTime
--- linked via noteables ---
noteables: [{noteable_type, noteable_id}]  (post|organization)
```

### Edition

```
id: UUID
county_id: UUID
title: Option<String>
period_start: NaiveDate
period_end: NaiveDate
status: String (draft|generated|review|approved|published|archived)
published_at: Option<DateTime>
created_at: DateTime
--- nested ---
rows: [EditionRow]
sections: [EditionSection]
```

### Tag

```
id: TagId (UUID)
kind: String (slug from tag_kinds)
value: String
display_name: Option<String>
parent_tag_id: Option<UUID>
color: Option<String>
description: Option<String>
emoji: Option<String>
```

### Member

```
id: UUID
expo_push_token: Option<String>
searchable_text: Option<String>
latitude: Option<f64>
longitude: Option<f64>
location_name: Option<String>
active: bool
notification_count_this_week: i32
paused_until: Option<DateTime>
```

### Media

```
id: UUID
filename: String
content_type: String
size_bytes: i64
storage_key: String
url: String
alt_text: Option<String>
width: Option<i32>
height: Option<i32>
uploaded_by: Option<UUID>
created_at: DateTime
```

---

## Authentication

**Flow**: Phone OTP via Twilio -> JWT token

**Extractors** (Axum):
- `AdminUser` - Requires JWT + admin role (403 if not admin)
- `AuthenticatedUser` - Requires valid JWT (401 if missing)
- `OptionalUser` - Returns `Option<AuthUser>`, never rejects

**Token**: Extracted from `X-User-Token` header or `Authorization: Bearer` header. Contains `member_id`, `phone_number`, `is_admin`.

**Admin phones**: Configured via `ADMIN_IDENTIFIERS` env var (comma-separated).

---

## Server Dependencies (ServerDeps)

| Service | Type | Purpose |
|---|---|---|
| `db_pool` | `PgPool` | PostgreSQL (5 connections) |
| `ai` | `OpenAi` | GPT-5 / GPT-5-Mini for extraction, scoring, rewriting |
| `embedding_service` | `EmbeddingService` | OpenAI Ada for 1536-dim vectors |
| `twilio` | `TwilioAdapter` | OTP send/verify |
| `pii_detector` | `PiiDetector` | Regex-based PII scrubbing |
| `storage` | `S3StorageAdapter` | MinIO/R2/S3 file uploads |
| `jwt_service` | `JwtService` | Token sign/verify |
| `stream_hub` | `StreamHub` | Real-time SSE pub/sub |

---

## Migrations (188 total)

Key tables by migration phase:

**Foundation (1-50)**: extensions (pgvector, postgis), organizations, needs, tags, posts, identifiers, notifications, embeddings, scrape_jobs

**Refactoring (31-87)**: need -> listing -> post renames, website crawling, agents, jobs infrastructure

**AI/Agent System (52-113)**: agents, search topics, page summaries, deduplication, contacts, providers

**Organizations v2 (141-170)**: organization rebuild, social profiles, notes, noteables, checklist, proposal comments

**Editions/Broadsheet (172-188)**: post types (Signal/Editorial split), edition system, row templates, widgets, sections, zip_codes, counties, post weights, urgency levels

---

## Frontend GraphQL Operations

Located in `packages/admin-app/lib/graphql/`:

| File | Fragments | Queries | Mutations |
|---|---|---|---|
| `fragments.ts` | PostListFields, PostDetailFields, OrganizationFields, NoteFields | - | - |
| `posts.ts` | - | PostStats, PostsList, SignalPosts, EditorialPosts, PostDetail, PostDetailFull | Approve, Reject, Archive, Delete, Reactivate, AddTag, RemoveTag, Regenerate, RegenerateTags, UpdateCapacity, Create, Update |
| `organizations.ts` | - | OrgsList, OrgDetail, OrgDetailFull, OrgChecklist | Create, Update, Delete, Approve, Reject, Suspend, SetStatus, ToggleChecklist, Regenerate |
| `tags.ts` | - | TagKinds, Tags | CreateKind, UpdateKind, DeleteKind, CreateTag, UpdateTag, DeleteTag |
| `notes.ts` | - | NotesList, NoteDetail, EntityNotes, OrgPosts | Create, Update, Delete, Link, Unlink, AutoAttach |
| `editions.ts` | - | Counties, CountyDashboard, EditionsList, LatestEditions, EditionHistory, EditionDetail, RowTemplates, PostTemplates, EditionKanbanStats | Create, Generate, Review, Approve, Publish, Archive, BatchGenerate, BatchApprove, BatchPublish, ReorderRows, AddRow, DeleteRow, AddPost, RemovePost, ChangeSlotTemplate, MoveSlot, AddWidget, UpdateWidget, RemoveWidget, AddSection, UpdateSection, ReorderSections, DeleteSection, AssignRowToSection |
| `media.ts` | - | MediaLibrary, PresignedUpload | ConfirmUpload, DeleteMedia |
| `dashboard.ts` | - | Dashboard | - |
