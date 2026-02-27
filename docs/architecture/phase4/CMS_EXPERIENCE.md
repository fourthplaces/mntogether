# Phase 4.0: CMS Experience (Decap-Inspired)

**Status:** Plan
**Priority:** 0 (design vision — informs all Phase 4 implementation)
**Depends on:** Phases 1–3 complete

---

## Context

The admin app at `/admin` is functional but minimal: flat list views, read-only post detail, no content authoring, no media management, no workflow visualization. Decap CMS (https://github.com/decaporg/decap-cms) provides a proven editorial UX — split-pane editors, kanban workflow boards, a media library, organized collection views, and consistent field chrome — used by thousands of content teams.

This document describes a comprehensive CMS experience for Root Editorial that adopts Decap's UX patterns while building everything natively in the existing stack. It sits alongside the other Phase 4 documents (Story Editor, Edition Cockpit, Signal Inbox, Email Newsletter) and provides the design vision they should follow.

---

## Design Philosophy: Harmony, Not Importation

The goal is harmony between the existing admin app's warm earth-tone design system and Decap's editorial patterns. Every component is built with the existing stack:

| Decap Uses | Root Editorial Uses |
|---|---|
| Redux + Immutable.js | React state + URQL |
| Emotion CSS-in-JS | Tailwind CSS v4 + warm-earth theme (`app/themes/warm-earth.css`) |
| SPA with React Router | Next.js App Router (`app/admin/(app)/`) |
| Git-based persistence | GraphQL → Restate → PostgreSQL |
| Custom Slate v0.47 editor | Plate.js v52+ (modern Slate, shadcn/ui components) |
| `react-split-pane` (unmaintained) | CSS Grid + drag handle (zero dependencies) |
| Custom DnD namespace system | `@dnd-kit` (accessible, maintained, Decap adopted for newer widgets) |
| Widget registry + EditorControl wrapper | Plate.js plugin system + shared `FieldWrapper` component |

### What We Adopt from Decap

| Decap Pattern | Root Editorial Adaptation |
|---|---|
| Split-pane editor with synchronized preview | Plate.js editor (left) + ReactMarkdown preview (right) in resizable panes |
| 3-column kanban workflow board | Draft → In Review → Published using existing post status column |
| Media library as full-screen modal | Dialog-based modal with upload zone, search, responsive grid |
| Collection sidebar with nesting + counts | Enhanced `AdminSidebar` with collapsible post types and badge counts |
| Entry cards with image previews + status badges | Enhanced cards for kanban and collection views |
| Consistent field chrome (label, hint, validation) | Shared `FieldWrapper` component wrapping all form fields |
| Configurable formatting toolbar | Plate.js `FixedToolbar` with composable button components |
| localStorage for editor preferences | Persist split ratio, preview visibility, sidebar collapsed groups |

### What We Don't Adopt

- **Redux** — React state + URQL cache is sufficient for this app's complexity
- **Immutable.js** — plain JS objects throughout
- **Infinite scroll** — stick with offset pagination (`PaginationControls` component)
- **Custom widget registry** — Plate.js plugins handle editor extensibility natively
- **Iframe-isolated preview** — ReactMarkdown in a styled `<div>` is simpler and shares CSS
- **`react-split-pane`** — unmaintained; CSS Grid + pointer events is trivial to build
- **Percentage-based scroll sync** — Decap's own issues acknowledge this is imprecise; skip it

---

## Visual Design: Blending Warm-Earth with Decap

Decap CMS has a clean, minimal aesthetic: white/gray surfaces, blue active states, purple/brown/green status coding, system font stack, 5px radii, and subtle shadows. The warm-earth theme has richer texture: beige surfaces, stone/amber accents, warm borders, and a layered shadow system.

The blend takes Decap's spatial patterns (dense toolbars, split panes, card grids, kanban columns) and dresses them in warm-earth's palette. The result should feel like Decap's competent editorial UX, but warmer and more intentional.

### Color Mapping: Decap → Warm-Earth

| Decap Element | Decap Color | Warm-Earth Equivalent | Token |
|---|---|---|---|
| Page background | `#f2f5f8` (cool gray) | `#E8E2D5` (warm beige) | `--color-surface` |
| Card/panel background | `#ffffff` | `#FFFFFF` | `--color-surface-raised` |
| Toolbar background | `#ffffff` | `#FDFCFA` (off-white) | `--color-surface-subtle` |
| Active/selected accent | `#4863c6` (blue) | `#D97706` (amber) | `--color-admin-accent` |
| Primary text | `#313d3e` | `#3D3D3D` | `--color-text-primary` |
| Secondary text | `#7b8290` | `#7D7D7D` | `--color-text-muted` |
| Field labels | `#7b8290` uppercase | `#A09A8D` | `--color-text-label` |
| Borders | `#dfdfe3` | `#E8DED2` | `--color-border` |
| Hover border | `#c7c7c7` | `#C4B8A0` | `--color-border-strong` |
| Focus ring | blue box-shadow | `#C4B8A0` ring-2 | `--color-focus-ring` |

### Workflow Status Colors

Decap uses purple/brown/green for its 3-column kanban. Map these to warm-earth's palette + the existing pathway card colors:

| Status | Decap Color | Warm-Earth Adaptation | Rationale |
|---|---|---|---|
| **Draft** | Purple (`#8b60ed`) | Lavender pathway (`#C4BAD4` bg, `#6B5B8A` text) | Draft = work in progress, creative space |
| **In Review** | Brown/Yellow (`#dca558`) | Warm pathway (`#F4D9B8` bg, `#854D0E` text) | Review = attention needed, warm alert |
| **Published** | Green (`#64d09e`) | Sage pathway (`#B8CFC4` bg, `#166534` text) | Published = live, healthy, active |
| **Rejected** | (not in Decap) | Danger semantic (`#FEE2E2` bg, `#991B1B` text) | Rejected = negative action |
| **Archived** | (not in Decap) | Muted surface (`#F5F1E8` bg, `#7D7D7D` text) | Archived = faded, not active |

These map onto `Badge` component variants. The pathway colors (`--color-pathway-warm`, `--color-pathway-sage`, `--color-pathway-lavender`) are already defined in `warm-earth.css` and give the kanban columns a distinctive warm feel that Decap's cooler palette lacks.

### New Theme Tokens (add to `warm-earth.css`)

```css
/* ===== EDITOR-SPECIFIC ===== */
--color-editor-bg: #FFFFFF;            /* Editor writing surface — crisp white */
--color-editor-gutter: #FDFCFA;        /* Line numbers, toolbar bg — subtle off-white */
--color-editor-selection: #F5F1E8;     /* Text selection highlight — warm muted */
--color-editor-divider: #C4B8A0;       /* Split pane divider — matches border-strong */
--color-editor-divider-active: #D97706; /* Divider while dragging — amber accent */

/* ===== TOOLBAR ===== */
--color-toolbar-bg: #FDFCFA;
--color-toolbar-border: #E8DED2;
--color-toolbar-button-hover: #F5F1E8;
--color-toolbar-button-active: #E8E2D5; /* Active formatting state (e.g., bold is on) */

/* ===== MEDIA LIBRARY ===== */
--color-media-checkerboard: #F0EBE0;   /* Transparency checkerboard for image previews */
--color-media-selected-ring: #D97706;  /* Amber ring on selected media card */

/* ===== KANBAN ===== */
--color-kanban-draft-bg: #F3EFF8;      /* Light lavender for Draft column background */
--color-kanban-review-bg: #FDF6EE;     /* Light warm for In Review column background */
--color-kanban-published-bg: #F0F7F3;  /* Light sage for Published column background */
--color-kanban-drop-highlight: #FEF3CD; /* Amber glow when dragging over a column */
```

### Component Styling Specifics

**Editor toolbar** (inspired by Decap's, dressed in warm-earth):
- Background: `--color-toolbar-bg` (off-white, not pure white — warmer than Decap)
- Bottom border: `1px solid --color-toolbar-border`
- Buttons: 32px square, `rounded-md`, transparent bg → `--color-toolbar-button-hover` on hover
- Active state: `--color-toolbar-button-active` bg + `--color-text-primary` text (Decap uses blue; we use the warm muted surface)
- Separator: 1px `--color-border-subtle` vertical line between button groups
- Tooltip: `bg-text-primary text-text-on-action` (dark tooltip, white text — matches existing admin style)

**Split pane divider**:
- Default: 4px wide, `--color-editor-divider` (stone)
- Hover: widens to 6px, `--color-editor-divider-active` (amber) — gives tactile feedback
- Cursor: `col-resize`
- Transition: `width 150ms, background-color 150ms`

**Kanban columns**:
- Each column has a subtle tinted background matching its status (lavender, warm, sage)
- Column headers: bold text + count badge using the status Badge variant
- Cards: white `--color-surface-raised` with `--shadow-card`, lift to `--shadow-card-hover` on hover
- Drop target highlight: `--color-kanban-drop-highlight` border glow (2px amber)

**Media library modal**:
- Full-screen overlay using existing `Dialog` component pattern
- Grid cards: white surface, `rounded-lg`, `--shadow-sm`
- Selected state: `ring-2` with `--color-media-selected-ring` (amber)
- Upload zone: dashed border `--color-border-strong`, turns `--color-admin-accent` on dragover
- Thumbnail background: checkerboard pattern for transparency using `--color-media-checkerboard`

**Field wrapper** (inspired by Decap's EditorControl):
- Label: `text-text-label`, `text-xs`, `font-semibold`, `uppercase`, `tracking-wider` (matches sidebar group labels)
- Optional indicator: `text-text-muted` "(optional)" suffix
- Error: `text-danger-text`, `text-xs`, right-aligned (same position as Decap)
- Hint: `text-text-muted`, `text-xs`, below the field (supports markdown like Decap's)
- Spacing: `mb-6` between fields (Decap uses 36px, this is 24px — slightly tighter to match the admin app's density)

**Workflow cards** (inspired by Decap's WorkflowCard):
- White card with `rounded-lg`, `--shadow-sm`
- Drag handle: 6 dots pattern in `--color-text-faint`, turns `--color-text-muted` on hover
- Title: `text-text-primary`, `font-medium`, `text-sm`, truncated with ellipsis
- Metadata badges: existing `Badge` component with appropriate variant
- Date: `text-text-muted`, `text-xs`
- Hover state: `--shadow-card` (lift effect), reveal action buttons

### Typography in the Editor

The editor writing surface should feel like a document, not a form:

- **Body text**: 16px, `text-text-body`, line-height 1.75 (generous for readability)
- **Headings**: `text-text-primary`, H1 = 28px/bold, H2 = 22px/semibold, H3 = 18px/semibold
- **Code blocks**: `bg-surface-muted`, `text-text-primary`, monospace, `rounded-md`, 1px `border-border`
- **Blockquotes**: 3px left border in `--color-admin-accent`, `pl-4`, `text-text-secondary`, italic
- **Links**: `--color-link` (#8B6D3F, warm brown — not blue like Decap)
- **Images**: `rounded-md`, `--shadow-sm`, max-width 100%, centered

This matches the existing `prose prose-stone` styling used on the post detail page, ensuring consistency between the editor preview and the read-only view.

---

## Architecture Decisions

### 1. Plate.js for the markdown editor

Plate.js v52+ is built on Slate.js (same foundation Decap uses) but with a modern API, 50+ plugins, and first-class Tailwind/shadcn support. Key capabilities:

- **Markdown round-tripping** via `@platejs/markdown` using remark/mdast — identical pipeline to Decap's serializers
- **Load from DB**: `editor.api.markdown.deserialize(post.descriptionMarkdown)`
- **Save to DB**: `editor.api.markdown.serialize()` → store as `description_markdown`
- **Plugin composition**: `BasicBlocksKit` (headings, blockquote, HR), `BasicMarksKit` (bold, italic, underline, strikethrough, code), `LinkPlugin`, `TablePlugin`, `CodeBlockPlugin` (with syntax highlighting via lowlight), `ImagePlugin`, `MarkdownPlugin`
- **UI components** install into `components/ui/` via shadcn CLI — same pattern as existing Button, Card, etc.
- **Editor pages** need `'use client'` directive. Server-side markdown processing uses `createSlateEditor` from base `platejs` (no `/react` imports).

**Why not MDXEditor**: Plate.js provides more control over the editing experience, uses the same Slate foundation as Decap (so patterns translate directly), and its headless shadcn architecture integrates cleanly with the existing Tailwind design system.

### 2. Split-pane layout with CSS Grid

Decap uses `react-split-pane` (unmaintained, last release 2020). Instead, build a resizable layout using:

```
CSS Grid: grid-template-columns: ${leftPct}fr 4px ${rightPct}fr
```

The 4px divider element handles `onPointerDown` → track `onPointerMove` → update column ratio. Persist ratio in `localStorage`. Three view modes (matching Decap):

1. **Editor only** — single column, no preview
2. **Editor + Preview** — resizable split (default 50/50)
3. **Preview only** — for reviewing rendered output

Toggle buttons in the toolbar switch modes.

### 3. Post statuses gain a "draft" state

Currently admin-created posts go straight to `active` (published). For the kanban workflow to make sense, we need a `draft` status:

| Status | Meaning | Visible publicly? |
|---|---|---|
| `draft` | Editor is working on it | No |
| `pending_approval` | Submitted externally (Signal, etc.), awaiting review | No |
| `active` | Published and visible | Yes |
| `rejected` | Declined during review | No |
| `archived` | Removed from active view | No |

**No migration needed** — the `status` column is `text` with no CHECK constraint. The `'draft'` value works immediately.

This changes the STORY_EDITOR.md decision that admin-created posts skip approval. Instead: admin creates a draft → edits it → explicitly publishes. The kanban makes this workflow visible.

### 4. MinIO for local dev, S3/R2 for production

Media uploads need object storage. Architecture:

- **`BaseStorageService` trait** in `kernel/traits.rs` (same pattern as `BaseTwilioService`)
- **`S3StorageAdapter`** using `aws-sdk-s3` crate (S3-compatible API works with MinIO, R2, and S3)
- **MinIO** service added to `docker-compose.yml` for local dev (S3-compatible, zero config)
- **Presigned upload URLs** — browser uploads directly to storage, then confirms with the API. No file data flows through the Rust server.

Upload flow:
```
Browser → GraphQL (presignedUpload) → Restate → S3 presign
Browser → S3 PUT (direct upload with presigned URL)
Browser → GraphQL (confirmUpload) → Restate → create media DB record
```

### 5. Media library as a modal overlay

Following Decap's pattern: the media library is a full-screen `Dialog` overlay (not a separate page route, though a standalone `/admin/media` page also exists for browsing). This keeps the editor context visible behind the backdrop.

Integration with Plate.js:
- Configure `ImagePlugin` with `disableUploadInsert: true` (prevents default drag-drop upload)
- Clicking "Image" in the toolbar opens the `MediaLibrary` modal
- On selecting an image, call `editor.tf.insert.image({ url: selectedMedia.url })`

### 6. Standalone `media` table, decoupled from posts

The existing `post_media` table (migration 000173) ties images to specific posts. The media library needs a standalone table:

- `media` table stores all uploaded files (filename, content type, size, S3 key, public URL, dimensions)
- `post_media.image_url` references `media.url` when linking media to posts
- Same image can appear in multiple posts
- Media library shows all uploads regardless of post association

### 7. Enhanced sidebar with content type nesting

The current `AdminSidebar` (`components/admin/AdminSidebar.tsx`) has flat `NavItem[]` groups. Enhance with:

```
Overview
  Dashboard
Content
  ▾ Posts (247)
      Stories (82)
      Notices (45)
      Exchanges (38)
      Events (42)
      Spotlights (22)
      References (18)
  Workflow Board
  Inbox (14)
  Editions (87)
  Media
Sources
  Organizations (12)
System
  Jobs
  Tags
```

Post type sub-items use URL query params (e.g., `/admin/posts?postType=story`) rather than separate routes. The sidebar's `NavItem` type gains an optional `children` array and `badge` count. Collapsed groups persist in `localStorage` (sidebar already persists its own collapsed state).

### 8. @dnd-kit for kanban drag-and-drop

`@dnd-kit/core` + `@dnd-kit/sortable` for the workflow board. Features:

- `DndContext` wraps the three columns
- Each column is a `SortableContext` + `useDroppable`
- Cards use `useSortable` for drag handles
- `DragOverlay` renders a ghost card during drag (smooth UX)
- Dropping in a different column triggers a status-change mutation

---

## Database Changes

### No migration for draft status

The `status` column on `posts` is `text` with no CHECK constraint. Using `'draft'` requires zero schema changes.

### New migration: `000179_create_media_library.sql`

```sql
CREATE TABLE media (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    storage_key TEXT NOT NULL,      -- S3/R2 object key (e.g., "media/2026/02/abc123.jpg")
    url TEXT NOT NULL,              -- Public/CDN URL
    alt_text TEXT,
    width INT,
    height INT,
    uploaded_by TEXT,               -- Admin identifier
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_media_created_at ON media(created_at DESC);
CREATE INDEX idx_media_content_type ON media(content_type);
```

The existing `post_media` table (migration 000173) remains unchanged. Its `image_url` column can store URLs from the new `media` table.

---

## Backend Changes

### Storage service trait: `packages/server/src/kernel/traits.rs`

New trait following the `BaseTwilioService` pattern (line 30):

```rust
#[async_trait]
pub trait BaseStorageService: Send + Sync {
    /// Upload bytes directly. Returns the public URL.
    async fn upload(&self, key: &str, data: Vec<u8>, content_type: &str) -> Result<String>;

    /// Delete an object by key.
    async fn delete(&self, key: &str) -> Result<()>;

    /// Generate a presigned PUT URL for direct browser upload.
    async fn presigned_upload_url(
        &self,
        key: &str,
        content_type: &str,
        expires_secs: u64,
    ) -> Result<String>;

    /// Construct the public URL for a given key (no network call).
    fn public_url(&self, key: &str) -> String;
}
```

### Storage adapter: `packages/server/src/kernel/deps.rs`

Add to `ServerDeps` (line 57):

```rust
pub storage: Arc<dyn BaseStorageService>,
```

New `S3StorageAdapter` struct using `aws-sdk-s3`:
- Configured from env vars: `S3_ENDPOINT`, `S3_BUCKET`, `S3_REGION`, `S3_ACCESS_KEY`, `S3_SECRET_KEY`, `S3_PUBLIC_URL`
- For MinIO: `S3_ENDPOINT=http://localhost:9000`, `S3_PUBLIC_URL=http://localhost:9000/media`
- For production R2/S3: standard AWS configuration

### New domain: `packages/server/src/domains/media/`

```
domains/media/
├── mod.rs
├── models/
│   └── media.rs          -- Media struct, CRUD, list_paginated
├── activities/
│   ├── mod.rs
│   └── core.rs           -- presign_upload, confirm_upload, delete_media
└── restate/
    └── services/
        └── media.rs       -- MediaService trait + impl
```

**Model** (`media.rs`):
- `Media` struct with `FromRow` derive
- `Media::create(filename, content_type, size_bytes, storage_key, url, alt_text, width, height, uploaded_by, pool)`
- `Media::find_by_id(id, pool)`
- `Media::list_paginated(filters, limit, offset, pool)` — supports content_type filter, returns `(Vec<Media>, i64)`
- `Media::delete(id, pool)`

**Activities** (`core.rs`):
- `presign_upload(filename, content_type, size_bytes, deps)` — generates storage key, calls `deps.storage.presigned_upload_url()`, returns `(presigned_url, storage_key, public_url)`
- `confirm_upload(storage_key, filename, content_type, size_bytes, alt_text, uploaded_by, deps)` — creates `Media` record in DB
- `delete_media(media_id, deps)` — deletes from S3 + DB

**Restate service** (`media.rs`):

```rust
#[restate_sdk::service]
pub trait MediaService {
    async fn presigned_upload(req: PresignedUploadRequest) -> Result<PresignedUploadResult, HandlerError>;
    async fn confirm_upload(req: ConfirmUploadRequest) -> Result<MediaResult, HandlerError>;
    async fn list(req: ListMediaRequest) -> Result<MediaListResult, HandlerError>;
    async fn delete(req: DeleteMediaRequest) -> Result<(), HandlerError>;
}
```

Register in `server.rs` with `.bind(MediaServiceImpl::with_deps(deps.clone()).serve())`.

### Post model updates

These overlap with STORY_EDITOR.md — whichever is implemented first handles them:

**`packages/server/src/domains/posts/models/post.rs`:**

1. Add `description_markdown` to `CreatePost` builder (after line 361):
   ```rust
   #[builder(default)]
   pub description_markdown: Option<String>,
   ```
   Update `Post::create` SQL (line 844) to include `description_markdown` as the 19th column.

2. Add to `PostFilters` (line 428):
   ```rust
   pub post_type: Option<&'a str>,
   pub submission_type: Option<&'a str>,
   ```
   Update `find_paginated` (line 565), `find_paginated_near_zip` (line 1474), and `count_by_status` (line 1098) with `($N::text IS NULL OR p.post_type = $N)` and `($N::text IS NULL OR p.submission_type = $N)` clauses.

**`packages/server/src/domains/posts/activities/core.rs`:**

3. New `admin_create_post` function — builds `CreatePost` with `status = "draft"`, `submission_type = "admin"`. Strips markdown to plain text for `description` field.
4. New `admin_update_post` function — uses existing `Post::update_content` (line 933) which already handles COALESCE for all content fields.
5. New `strip_markdown` helper — regex-based or using `pulldown-cmark` crate.

**`packages/server/src/domains/posts/restate/services/posts.rs`:**

6. Add `post_type: Option<String>` and `submission_type: Option<String>` to `ListPostsRequest` (line 30).
7. New `CreatePostRequest` type + `create_post` handler on `PostsService`.

**`packages/server/src/domains/posts/restate/virtual_objects/post.rs`:**

8. New `UpdatePostContentRequest` type + `update_content` exclusive handler on `Post` VO. Separate from the existing `EditApproveRequest` (line 46) — purely content updates, no status change.

---

## GraphQL Changes

### Schema: `packages/shared/graphql/schema.ts`

**Media types:**

```graphql
type Media {
  id: ID!
  filename: String!
  contentType: String!
  sizeBytes: Int!
  url: String!
  altText: String
  width: Int
  height: Int
  createdAt: String!
}

type MediaConnection {
  media: [Media!]!
  totalCount: Int!
  hasNextPage: Boolean!
}

input UploadMediaInput {
  filename: String!
  contentType: String!
  sizeBytes: Int!
  altText: String
}

type PresignedUpload {
  uploadUrl: String!
  mediaId: ID!
  publicUrl: String!
}
```

**Post additions:**

```graphql
input CreatePostInput {
  title: String!
  descriptionMarkdown: String!
  summary: String
  postType: String
  weight: String
  priority: Int
  urgency: String
  location: String
  organizationId: ID
}

input UpdatePostInput {
  title: String
  descriptionMarkdown: String
  summary: String
  postType: String
  weight: String
  priority: Int
  urgency: String
  location: String
}
```

**Add to `Post` type:**
```graphql
submissionType: String
```

**Add to `posts` query args:**
```graphql
submissionType: String
```

**Add to `Query`:**
```graphql
mediaLibrary(limit: Int, offset: Int, contentType: String): MediaConnection!
presignedUpload(input: UploadMediaInput!): PresignedUpload!
```

**Add to `Mutation`:**
```graphql
createPost(input: CreatePostInput!): Post!
updatePost(id: ID!, input: UpdatePostInput!): Post!
confirmUpload(mediaId: ID!, altText: String, width: Int, height: Int): Media!
deleteMedia(id: ID!): Boolean!
```

### Resolver: `packages/shared/graphql/resolvers/media.ts` (new file)

```typescript
export const mediaResolvers = {
  Query: {
    mediaLibrary: async (_parent, args, ctx) => {
      return ctx.restate.callService("Media", "list", {
        limit: args.limit,
        offset: args.offset,
        content_type: args.contentType,
      });
    },
    presignedUpload: async (_parent, args, ctx) => {
      return ctx.restate.callService("Media", "presigned_upload", {
        filename: args.input.filename,
        content_type: args.input.contentType,
        size_bytes: args.input.sizeBytes,
        alt_text: args.input.altText,
      });
    },
  },
  Mutation: {
    confirmUpload: async (_parent, args, ctx) => {
      return ctx.restate.callService("Media", "confirm_upload", { ... });
    },
    deleteMedia: async (_parent, args, ctx) => {
      return ctx.restate.callService("Media", "delete", { id: args.id });
    },
  },
};
```

Register in `packages/shared/graphql/resolvers/index.ts` (add `mediaResolvers` to `mergeResolvers` array).

### Post resolver updates: `packages/shared/graphql/resolvers/post.ts`

Add `createPost` and `updatePost` mutations (same as STORY_EDITOR.md). Thread `submissionType` through the `posts` query resolver.

---

## Frontend Changes

### Feature 1: Editor Experience (Plate.js)

**Install:**
```bash
cd packages/admin-app
yarn add platejs @platejs/basic-nodes @platejs/markdown @platejs/link @platejs/table @platejs/code-block @platejs/media remark-gfm
npx shadcn@latest add @plate/editor-basic @plate/fixed-toolbar
```

#### New: `packages/admin-app/components/admin/FieldWrapper.tsx`

Consistent field chrome for all form fields, inspired by Decap's `EditorControl`:

```
┌──────────────────────────────────────┐
│ Label                    (optional)  │  ← label row
│ ┌──────────────────────────────────┐ │
│ │        [Widget/Input]            │ │  ← field content (children)
│ └──────────────────────────────────┘ │
│ Hint text or validation error        │  ← hint/error row
└──────────────────────────────────────┘
```

Props: `label`, `hint?`, `error?`, `required?`, `children`. Uses existing text color tokens from `warm-earth.css` (`text-text-secondary` for labels, `text-danger` for errors, `text-text-muted` for hints).

#### New: `packages/admin-app/components/admin/PlateEditor.tsx`

`'use client'` component wrapping Plate.js:

- Plugins: `BasicBlocksKit`, `BasicMarksKit`, `LinkPlugin`, `TablePlugin`, `CodeBlockPlugin` (with lowlight), `ImagePlugin` (with `disableUploadInsert: true`), `MarkdownPlugin` (with `remarkGfm`)
- `FixedToolbar` with buttons: Bold, Italic, Strikethrough, Code, Link, H1–H3, Blockquote, Bulleted List, Numbered List, Code Block, Image (opens media library), Table
- Props: `initialMarkdown: string`, `onChange: (markdown: string) => void`, `onImageInsert: () => void` (triggers media library modal)
- Loads initial value via `editor.api.markdown.deserialize(initialMarkdown)` in `usePlateEditor`'s async `value` callback
- Serializes via `editor.api.markdown.serialize()` in `onChange`

#### New: `packages/admin-app/components/admin/SplitPaneEditor.tsx`

Resizable editor + preview layout:

```
┌─────────────────────┬──┬─────────────────────┐
│                     │  │                     │
│   PlateEditor       │  │   ReactMarkdown     │
│   (WYSIWYG)         │▐▐│   (rendered preview) │
│                     │  │                     │
│                     │  │                     │
└─────────────────────┴──┴─────────────────────┘
                      ↑
               Draggable divider (4px)
```

- CSS Grid: `grid-template-columns: ${leftPct}% 4px 1fr`
- Divider: `cursor-col-resize`, `bg-stone-300 hover:bg-amber-400`, `onPointerDown` starts resize
- Three modes: editor-only, split (default), preview-only — toggle buttons in a toolbar row
- Split ratio stored in `localStorage('editor-split-ratio')`
- Preview side: `<ReactMarkdown className="prose prose-stone max-w-none">` (already used on post detail page)

#### New: `packages/admin-app/components/admin/PostForm.tsx`

Shared form component used by create and edit pages:

- **Title**: `<Input>` component inside `<FieldWrapper label="Title" required>`
- **Description**: `<SplitPaneEditor>` inside `<FieldWrapper label="Content" required>`
- **Summary**: `<Textarea>` inside `<FieldWrapper label="Summary" hint="Brief plain-text summary">`
- **Post Type**: `<select>` with 6 options (story, notice, exchange, event, spotlight, reference)
- **Weight**: `<select>` (heavy, medium, light)
- **Priority**: `<Input type="number">`
- **Urgency**: `<select>` (low, medium, high, urgent) — optional
- **Location**: `<Input>` — optional
- **Organization**: `<select>` from `OrganizationsListQuery` — optional

Props: `initialValues?: PostFormValues`, `onSubmit: (values) => Promise<void>`, `onCancel?: () => void`, `loading?: boolean`

#### New: `packages/admin-app/app/admin/(app)/posts/new/page.tsx`

- `PostForm` with empty initial values
- On submit: calls `CreatePostMutation` → post saved as `status='draft'`
- On success: redirect to `/admin/posts/[id]`
- Header: "New Post" with `BackLink` to `/admin/posts`

#### Modified: `packages/admin-app/app/admin/(app)/posts/[id]/page.tsx`

- Add "Edit" button to the header actions row
- When `isEditing` state is true: replace read-only content with `PostForm` pre-filled from post data
- Save calls `UpdatePostMutation`, exits edit mode, refetches data
- Cancel reverts to read-only view
- Existing approve/reject/archive actions remain in read-only mode

#### Modified: `packages/admin-app/app/admin/(app)/posts/page.tsx`

- Add "New Post" button (Button variant="primary") in page header, links to `/admin/posts/new`

### Feature 2: Workflow Board (Kanban)

**Install:**
```bash
cd packages/admin-app
yarn add @dnd-kit/core @dnd-kit/sortable @dnd-kit/utilities
```

#### New: `packages/admin-app/components/admin/WorkflowCard.tsx`

Compact card for the kanban board:

```
┌──────────────────────────────────┐
│ ⠿  Volunteers Needed: North...  │  ← drag handle + truncated title
│ exchange · high · Feb 24         │  ← type badge, urgency badge, date
│                        [Edit ▸]  │  ← hover action
└──────────────────────────────────┘
```

- Drag handle (grip dots icon) on the left
- Title: truncated, clickable → links to `/admin/posts/[id]`
- Metadata row: post type `Badge`, urgency `Badge` (if present), relative date
- Hover: reveal "Edit" link and "Reject" button (danger ghost)
- Uses `useSortable` from `@dnd-kit/sortable`
- Styling: `Card` component with `variant="interactive"`, stone/amber palette

#### New: `packages/admin-app/components/admin/WorkflowColumn.tsx`

Drop target column:

```
┌─────────────────┐
│ Drafts (8)      │  ← header with count
├─────────────────┤
│ [WorkflowCard]  │
│ [WorkflowCard]  │
│ [WorkflowCard]  │
│ ...             │
│                 │  ← drop zone
└─────────────────┘
```

- Header: status label + count badge
- Scrollable card list
- `useDroppable` from `@dnd-kit/core`
- Visual feedback: border highlight when dragging over

#### New: `packages/admin-app/app/admin/(app)/workflow/page.tsx`

Three-column kanban board:

```
┌─────────────────────────────────────────────────────────┐
│  Workflow Board                              [Type ▼]   │
├─────────────────┬─────────────────┬─────────────────────┤
│  Drafts (8)     │  In Review (14) │  Published (156)    │
│                 │                 │                     │
│  WorkflowCard   │  WorkflowCard   │  WorkflowCard       │
│  WorkflowCard   │  WorkflowCard   │  WorkflowCard       │
│  WorkflowCard   │  WorkflowCard   │  WorkflowCard       │
│  ...            │  ...            │  ...                │
└─────────────────┴─────────────────┴─────────────────────┘
```

- **Columns**: Drafts (`status=draft`), In Review (`status=pending_approval`), Published (`status=active`)
- **Data**: Three parallel `posts()` queries filtered by status
- **Drag behavior**:
  - Draft → Published: calls `ApprovePostMutation` (sets `status=active`, `published_at=NOW()`)
  - In Review → Published: same approve flow
  - In Review → Drafts: could map to "claim for editing" (sets `status=draft`, `submission_type=admin`)
  - Any → Rejected: via card menu action (calls `RejectPostMutation`)
- **DragOverlay**: renders a semi-transparent ghost `WorkflowCard` during drag
- **Filter**: dropdown to filter by post type
- **Responsive**: columns stack vertically on mobile

#### New: `packages/admin-app/lib/graphql/workflow.ts`

```typescript
export const WorkflowDraftsQuery = graphql(`
  query WorkflowDrafts($postType: String, $limit: Int, $offset: Int) {
    posts(status: "draft", postType: $postType, limit: $limit, offset: $offset) {
      posts { ...PostListFields }
      totalCount
    }
  }
`);

// Similar queries for pending_approval and active statuses
```

### Feature 3: Media Library

#### New: `packages/admin-app/components/admin/MediaCard.tsx`

Thumbnail card for the media grid:

```
┌──────────────────────┐
│ ┌──────────────────┐ │
│ │                  │ │  ← image preview (aspect-ratio: 4/3)
│ │    [thumbnail]   │ │
│ │                  │ │
│ └──────────────────┘ │
│ photo-2026-02.jpg    │  ← filename (truncated)
│ 245 KB · image/jpeg  │  ← size + content type
└──────────────────────┘
```

- Image with `loading="lazy"`, checkerboard background for transparency
- Selection ring: `ring-2 ring-amber-500` when selected
- Hover: show delete button (top-right corner)
- Non-image files: show file type icon instead of thumbnail
- Uses `Card` component with `variant="interactive"`

#### New: `packages/admin-app/components/admin/MediaUploadZone.tsx`

Drag-and-drop upload area:

```
┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
  Drop files here or click
│ to upload                   │
  Accepts: JPG, PNG, GIF, SVG
│ Max: 10 MB                  │
└ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘
```

- Dashed border, stone-300 (turns amber-500 on dragover)
- Hidden `<input type="file">` triggered on click
- On file select/drop:
  1. Call `PresignedUploadMutation` → get `{ uploadUrl, mediaId, publicUrl }`
  2. `PUT` file directly to `uploadUrl` (XMLHttpRequest for progress tracking)
  3. Call `ConfirmUploadMutation` with dimensions (read from `Image()` element)
  4. Append new `Media` to the grid

#### New: `packages/admin-app/components/admin/MediaLibrary.tsx`

Full-screen modal (extends `Dialog` component with `className="max-w-5xl h-[80vh]"`):

```
┌─────────────────────────────────────────────────────────┐
│  Media Library                              [× Close]   │
├─────────────────────────────────────────────────────────┤
│  [🔍 Search...]                     [Upload Files ↑]   │
├─────────────────────────────────────────────────────────┤
│ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐            │
│ │        │ │        │ │        │ │        │            │
│ │ thumb  │ │ thumb  │ │ thumb  │ │ thumb  │            │
│ │        │ │        │ │        │ │        │            │
│ │name.jpg│ │name.png│ │name.svg│ │name.gif│            │
│ └────────┘ └────────┘ └────────┘ └────────┘            │
│                                                         │
│ ┌────────┐ ┌────────┐                                   │
│ │        │ │        │                                   │
│ │ thumb  │ │ thumb  │    [Drop files here or click     │
│ │        │ │        │     to upload]                    │
│ │name.jpg│ │name.png│                                   │
│ └────────┘ └────────┘                                   │
├─────────────────────────────────────────────────────────┤
│  Showing 1–12 of 34           [Prev] [Next]   [Select] │
└─────────────────────────────────────────────────────────┘
```

- **Search**: filters by filename (client-side on loaded page, server-side across pages)
- **Grid**: responsive — `grid-cols-2 sm:grid-cols-3 lg:grid-cols-4`
- **Upload zone**: inline at the bottom of the grid (or shown as empty state)
- **Selection**: click to select, amber ring highlight. "Select" button in footer confirms choice.
- **Pagination**: `PaginationControls` at bottom
- Props: `isOpen`, `onClose`, `onSelect: (media: Media) => void`, `multiple?: boolean`

#### New: `packages/admin-app/app/admin/(app)/media/page.tsx`

Standalone media browsing page (same grid as the modal, but full-page layout):

- Header: "Media Library" with upload button
- Full-width responsive grid
- Bulk delete (checkbox selection)
- Content type filter dropdown
- Uses `MediaLibraryQuery`

#### New: `packages/admin-app/lib/graphql/media.ts`

```typescript
export const MediaLibraryQuery = graphql(`
  query MediaLibrary($limit: Int, $offset: Int, $contentType: String) {
    mediaLibrary(limit: $limit, offset: $offset, contentType: $contentType) {
      media { id filename contentType sizeBytes url altText width height createdAt }
      totalCount
      hasNextPage
    }
  }
`);

export const PresignedUploadMutation = graphql(`
  query PresignedUpload($input: UploadMediaInput!) {
    presignedUpload(input: $input) { uploadUrl mediaId publicUrl }
  }
`);

export const ConfirmUploadMutation = graphql(`
  mutation ConfirmUpload($mediaId: ID!, $altText: String, $width: Int, $height: Int) {
    confirmUpload(mediaId: $mediaId, altText: $altText, width: $width, height: $height) {
      id url filename
    }
  }
`);

export const DeleteMediaMutation = graphql(`
  mutation DeleteMedia($id: ID!) {
    deleteMedia(id: $id)
  }
`);
```

### Feature 4: Enhanced Sidebar

#### Modified: `packages/admin-app/components/admin/AdminSidebar.tsx`

Extend the `NavItem` and `NavGroup` types:

```typescript
interface NavItem {
  href: string;
  label: string;
  icon: React.ReactNode;
  badge?: number;           // NEW — count badge
  children?: NavItem[];     // NEW — nested sub-items
  queryParam?: string;      // NEW — appended as ?postType=value
}
```

Changes to the `navGroups` array:

1. **Posts** item gains `children` array with one sub-item per post type (Stories, Notices, Exchanges, Events, Spotlights, References). Each links to `/admin/posts?postType=story` etc.
2. **New items** under Content: "Workflow Board" (`/admin/workflow`), "Inbox" (`/admin/inbox` — for Signal Inbox phase), "Media" (`/admin/media`)
3. **Badge counts**: Posts shows total, Inbox shows pending count. Counts come from a lightweight sidebar query (piggybacking on dashboard data or a dedicated small query).

Collapsible behavior:
- Clicking a parent item with children toggles the children visibility
- Collapsed state stored in `localStorage('admin-sidebar-groups')`
- When sidebar is collapsed (icon-only mode), children are hidden

---

## Existing Code to Reuse

| What | Where | How |
|---|---|---|
| `PostReviewCard` | `components/admin/PostReviewCard.tsx` | Pattern for `WorkflowCard` design |
| `Dialog` | `components/ui/Dialog.tsx` | Base for `MediaLibrary` modal |
| `Badge` | `components/ui/Badge.tsx` | Status/type badges throughout |
| `Button` | `components/ui/Button.tsx` | All action buttons |
| `Card` | `components/ui/Card.tsx` | Media cards, workflow cards |
| `Input`, `Textarea` | `components/ui/Input.tsx` | Form fields in `PostForm` |
| `AdminLoader` | `components/admin/AdminLoader.tsx` | Loading states |
| `PaginationControls` | `components/ui/PaginationControls.tsx` | Media library, workflow pagination |
| `ApprovePostMutation` | `lib/graphql/posts.ts` | Workflow status changes (draft→active) |
| `RejectPostMutation` | `lib/graphql/posts.ts` | Rejection flow |
| `PostListFields` fragment | `lib/graphql/fragments.ts` | Workflow board card data |
| `PostDetailFields` fragment | `lib/graphql/fragments.ts` | Editor form pre-fill |
| `OrganizationsListQuery` | `lib/graphql/organizations.ts` | Organization dropdown in PostForm |
| `BaseTwilioService` pattern | `kernel/traits.rs:30` | Template for `BaseStorageService` |
| `TwilioAdapter` pattern | `kernel/deps.rs:25` | Template for `S3StorageAdapter` |
| `PostMedia` model | `posts/models/post_media.rs` | Reference for media-post linking |
| `warm-earth.css` tokens | `app/themes/warm-earth.css` | Colors, radii, shadows for new components |
| `cn()` utility | `lib/utils.ts` | Conditional Tailwind class composition |
| `useOffsetPagination` hook | `lib/hooks/` | Pagination state for media library |

---

## Docker Changes

### `docker-compose.yml` — Add MinIO service

```yaml
minio:
  image: minio/minio:latest
  command: server /data --console-address ":9001"
  ports:
    - "9000:9000"   # S3 API
    - "9001:9001"   # MinIO Console
  environment:
    MINIO_ROOT_USER: minioadmin
    MINIO_ROOT_PASSWORD: minioadmin
  volumes:
    - minio-data:/data
```

Add `minio-data` to the `volumes:` section.

### `.env.example` — Add storage config

```env
# Object Storage (S3-compatible — MinIO for local, R2/S3 for prod)
S3_ENDPOINT=http://localhost:9000
S3_BUCKET=media
S3_REGION=us-east-1
S3_ACCESS_KEY=minioadmin
S3_SECRET_KEY=minioadmin
S3_PUBLIC_URL=http://localhost:9000/media
```

### MinIO bucket initialization

Add a one-time setup script or Docker entrypoint that creates the `media` bucket:
```bash
mc alias set local http://localhost:9000 minioadmin minioadmin
mc mb local/media --ignore-existing
mc anonymous set download local/media
```

---

## Implementation Phases

These phases can be worked on independently (A and B in parallel, C and D after both):

### Phase A: Editor Core (unblocks everything)

1. Install Plate.js + dependencies in `packages/admin-app`
2. Build `FieldWrapper` component
3. Build `PlateEditor` component with plugins and toolbar
4. Build `SplitPaneEditor` with resizable panes and localStorage
5. Build `PostForm` component
6. Build `/admin/posts/new` page
7. Add edit mode to `/admin/posts/[id]` page
8. Add "New Post" button to `/admin/posts` page
9. **Backend**: `CreatePost` builder fix, `PostFilters` updates, activities, Restate handlers
10. **GraphQL**: mutations, resolvers, codegen

### Phase B: Media Library (parallel with Phase A)

11. Add MinIO to `docker-compose.yml` + `.env.example`
12. `BaseStorageService` trait + `S3StorageAdapter`
13. Migration `000179_create_media_library.sql`
14. `Media` model + activities + `MediaService` Restate service
15. GraphQL types + resolvers for media
16. Register `mediaResolvers` in `resolvers/index.ts`
17. Build `MediaCard`, `MediaUploadZone` components
18. Build `MediaLibrary` modal
19. Build `/admin/media` page
20. Wire Plate.js `ImagePlugin` → media library modal

### Phase C: Workflow Board (after Phase A)

21. Build `WorkflowCard` component
22. Build `WorkflowColumn` component
23. Build `/admin/workflow` page with @dnd-kit kanban
24. Wire drag-and-drop to status change mutations
25. Add workflow GraphQL queries

### Phase D: Enhanced Sidebar (after A + B + C)

26. Extend `NavItem` type with `children` and `badge`
27. Add nested post type items with counts
28. Add Workflow Board, Media, Inbox links with badges
29. Implement collapsible groups with localStorage persistence

---

## Verification

1. **Editor round-trip**: Create a post at `/admin/posts/new` with headings, bold, lists, links, code blocks, and an image. Save. Reload the page. Click Edit. Verify all content renders correctly in the Plate.js editor and the split-pane preview.

2. **Draft workflow**: New posts save as `draft`. Verify they appear in the Drafts column of `/admin/workflow`. Verify they do NOT appear on the public site.

3. **Media upload**: Open the media library. Upload a JPG. Verify it appears in the grid with thumbnail. Verify the file exists in MinIO (check via MinIO console at `:9001`).

4. **Media in editor**: In the post editor, click the Image toolbar button. Verify the media library modal opens. Select an image. Verify it inserts into the editor and renders in the preview.

5. **Kanban drag**: On `/admin/workflow`, drag a card from Drafts to Published. Verify the post status changes to `active`, `published_at` is set, and the post appears on the public site.

6. **Kanban: In Review → Published**: Seed a `pending_approval` post. Verify it appears in the In Review column. Drag to Published. Verify approval flow works.

7. **Sidebar navigation**: Expand the Posts section in the sidebar. Click "Stories". Verify posts list filters to `postType=story`. Verify badge counts match actual post totals.

8. **Persistence**: Set the editor split ratio to 70/30. Close the browser tab. Reopen. Verify the split ratio is restored. Collapse the Posts sidebar group. Reload. Verify it stays collapsed.

9. **Responsive**: Resize to mobile width. Verify the kanban columns stack vertically, the media grid reduces to 2 columns, and the editor hides the preview pane.
