# Phase 4.1: Markdown Story Editor

**Status:** Plan
**Priority:** 1 of 4 (highest — unblocks all other Phase 4 work)
**Depends on:** Nothing (Phases 1–3 complete)

---

## Context

The CMS has no way to create or edit posts. The post detail page at `/admin/posts/[id]` is read-only — it renders markdown content and provides approve/reject/archive actions, but no editing UI. There are no `createPost` or `updatePost` GraphQL mutations. The entire editorial pipeline (cockpit, inbox, newsletter) depends on editors being able to author and modify content.

---

## Architecture Decisions

### 1. Plate.js for WYSIWYG markdown editing

> **Updated by [CMS_EXPERIENCE.md](CMS_EXPERIENCE.md), Decision 1.** MDXEditor was the original choice; Plate.js supersedes it.

The target users are non-technical community editors. They need a toolbar-based editor, not a split-pane code view. Plate.js v52+ is built on Slate.js with a modern API, 50+ plugins, and first-class Tailwind/shadcn support. Key capabilities:

- **Markdown round-tripping** via `@platejs/markdown` using remark/mdast
- **Load from DB**: `editor.api.markdown.deserialize(post.descriptionMarkdown)`
- **Save to DB**: `editor.api.markdown.serialize()` → store as `description_markdown`
- **Plugin composition**: `BasicBlocksKit`, `BasicMarksKit`, `LinkPlugin`, `TablePlugin`, `CodeBlockPlugin`, `ImagePlugin`, `MarkdownPlugin`
- **UI components** install into `components/ui/` via shadcn CLI — same pattern as existing Button, Card, etc.

Alternatives considered:
- **MDXEditor**: Simpler, but Plate.js provides more control and uses the same Slate foundation as Decap CMS (patterns translate directly)
- **Tiptap**: Heavier, outputs HTML not markdown, requires a markdown serializer
- **@uiw/react-md-editor**: Split-pane view, less friendly for non-technical users

### 2. `description_markdown` is source of truth

The Post model already has both `description` (plain text) and `description_markdown` (markdown). The editor writes to `description_markdown`. On save, auto-generate plain text for `description` by stripping markdown syntax. The existing rendering in the post detail page already checks `descriptionMarkdown` first.

### 3. Separate create page, inline edit mode

- `/admin/posts/new` — dedicated creation page
- Edit mode toggle on `/admin/posts/[id]` — switches in-place between read-only and edit views

Both reuse a shared `PostForm` component.

### 4. Admin-created posts skip approval

Posts created via the admin editor get `submission_type = 'admin'` and `status = 'active'` (with `published_at = NOW()`). They don't need to go through the pending approval queue — the editor *is* the human approver.

### 5. PostForm is type-aware with field group defaults

> **From [CMS_SYSTEM_SPEC.md](../CMS_SYSTEM_SPEC.md) §5 and §9.2.**

When the editor selects a post type in the form, different field groups should be open by default. The 6 post types and their default field groups:

| Type | Default Field Groups | Form Character |
|------|---------------------|----------------|
| `story` | media, meta (kicker, byline) | Rich text editor dominates |
| `notice` | meta (timestamp), source | Short body + source attribution |
| `exchange` | contact, items, status | Structured form grid, body secondary |
| `event` | datetime, location, contact | Date/time pickers prominent |
| `spotlight` | person, media, location, contact | Profile assembly — structured fields |
| `reference` | items, contact, location, schedule, meta (updated) | Editable item table, body secondary |

Any field group can be toggled on/off regardless of type. Changing the type dropdown re-arranges which field groups are open, but doesn't discard data.

The initial implementation (below) covers universal fields only. Type-specific field group collapsibles are a follow-up tracked in [CMS_EXPERIENCE.md](CMS_EXPERIENCE.md).

### 6. `createPost` via PostsService, `updatePost` via Post virtual object

Creation goes through the stateless `PostsService` (no existing post key). Content updates go through the keyed `Post` virtual object to serialize writes per post, matching the existing `edit_approve` / `approve` / `reject` pattern.

### 7. All operations route through Restate

> **See [ARCHITECTURE_DECISIONS.md](../ARCHITECTURE_DECISIONS.md), Decision 4.**

`createPost` and `updatePost` route through Restate like everything else. `createPost` goes through the stateless `PostsService` (Decision 6). `updatePost` goes through the keyed `Post` virtual object, which serializes writes per post and matches the existing `edit_approve` / `approve` / `reject` pattern. GraphQL resolvers call `ctx.restate.callService(...)` and `ctx.restate.callObject(...)` — one consistent pattern for all backend operations.

---

## Database Changes

**No migration needed.** The `description_markdown` column already exists on the `posts` table. The `CreatePost` builder and `Post::create` SQL need updating to include it, but this is a code change, not a schema change.

---

## Backend Changes

### Model: `packages/server/src/domains/posts/models/post.rs`

**Add `description_markdown` to `CreatePost` builder** (line ~354):

```rust
// Current builder is missing description_markdown.
// Add after the `description` field:
#[builder(default)]
pub description_markdown: Option<String>,
```

**Add `description_markdown` to `Post::create` SQL** (line ~844):

The INSERT statement needs `description_markdown` added as the 19th column/parameter. The builder field binds after `published_at`.

**Add `post_type` and `submission_type` filters to `PostFilters`** (line ~428):

```rust
pub struct PostFilters<'a> {
    pub status: Option<&'a str>,
    pub source_type: Option<&'a str>,
    pub source_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub search: Option<&'a str>,
    pub post_type: Option<&'a str>,        // NEW
    pub submission_type: Option<&'a str>,   // NEW
}
```

Update `find_paginated`, `find_paginated_near_zip`, and `count_by_status` SQL to include these new filter predicates. Both queries already follow the `($N::text IS NULL OR p.column = $N)` pattern — add two more clauses.

### Activity: `packages/server/src/domains/posts/activities/core.rs`

Add two new functions:

```rust
/// Admin creates a post directly (skips approval, immediately active).
pub async fn admin_create_post(input: AdminCreatePostInput, deps: &ServerDeps) -> Result<Post> {
    let plain_text = strip_markdown(&input.description_markdown);
    let post = Post::create(
        CreatePost::builder()
            .title(input.title)
            .description(plain_text)
            .description_markdown(Some(input.description_markdown))
            .summary(input.summary)
            .post_type(input.post_type.unwrap_or("story".into()))
            .weight(input.weight.unwrap_or("medium".into()))
            .priority(input.priority.unwrap_or(0))
            .urgency(input.urgency)
            .location(input.location)
            .status("active".to_string())
            .submission_type(Some("admin".to_string()))
            .published_at(Some(Utc::now()))
            .build(),
        &deps.db_pool,
    ).await?;
    Ok(post)
}

/// Admin updates post content (resets relevance score).
pub async fn admin_update_post(post_id: PostId, input: AdminUpdatePostInput, deps: &ServerDeps) -> Result<Post> {
    let description = input.description_markdown.as_ref().map(|md| strip_markdown(md));
    Post::update_content(
        UpdatePostContent::builder()
            .id(post_id)
            .title(input.title)
            .description(description)
            .description_markdown(input.description_markdown)
            .summary(input.summary)
            .post_type(input.post_type)
            .weight(input.weight)
            .priority(input.priority)
            .urgency(input.urgency)
            .location(input.location)
            .build(),
        &deps.db_pool,
    ).await
}
```

Add a `strip_markdown` helper that removes markdown syntax to produce a plain-text `description`. This can be a simple regex-based strip (remove `#`, `*`, `[](...)`, etc.) or use the `pulldown-cmark` crate to parse and extract text nodes.

### Restate: `packages/server/src/domains/posts/restate/services/posts.rs`

Add to the `PostsService` trait and impl:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub description_markdown: String,
    pub summary: Option<String>,
    pub post_type: Option<String>,
    pub weight: Option<String>,
    pub priority: Option<i32>,
    pub urgency: Option<String>,
    pub location: Option<String>,
    pub organization_id: Option<Uuid>,
}
impl_restate_serde!(CreatePostRequest);

// Handler: create_post
// Calls activities::admin_create_post
// Returns PostResult
```

Also add `post_type` and `submission_type` fields to the existing `ListPostsRequest` (line ~30) and thread them through to `PostFilters` in the `list` handler.

### Restate: `packages/server/src/domains/posts/restate/virtual_objects/post.rs`

Add a new exclusive handler:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePostContentRequest {
    pub title: Option<String>,
    pub description_markdown: Option<String>,
    pub summary: Option<String>,
    pub post_type: Option<String>,
    pub weight: Option<String>,
    pub priority: Option<i32>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}
impl_restate_serde!(UpdatePostContentRequest);

// Handler: update_content (exclusive)
// Calls activities::admin_update_post
// Returns PostResult
```

Note: The existing `EditApproveRequest` (line ~46) already has similar fields but is coupled to the approval flow. The new handler is purely for content updates without changing status.

---

## GraphQL Changes

### Schema: `packages/shared/graphql/schema.ts`

Add input types and mutations:

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

Add to `Mutation`:
```graphql
createPost(input: CreatePostInput!): Post!
updatePost(id: ID!, input: UpdatePostInput!): Post!
```

Add to `posts` query args:
```graphql
posts(
  ...existing args...
  postType: String      # already exists
  submissionType: String  # NEW
): PostConnection!
```

Add to `Post` type:
```graphql
type Post {
  ...existing fields...
  submissionType: String  # NEW — needed for inbox filtering
  weight: Weight          # already exists
  priority: Int           # already exists
}
```

### Resolver: `packages/shared/graphql/resolvers/post.ts`

Add `createPost` and `updatePost` mutation resolvers:

```typescript
createPost: async (_parent, args: { input: CreatePostInput }, ctx) => {
  const result = await ctx.restate.callService("Posts", "create_post", {
    title: args.input.title,
    description_markdown: args.input.descriptionMarkdown,
    summary: args.input.summary,
    post_type: args.input.postType,
    weight: args.input.weight,
    priority: args.input.priority,
    urgency: args.input.urgency,
    location: args.input.location,
    organization_id: args.input.organizationId,
  });
  return result;
},

updatePost: async (_parent, args: { id: string; input: UpdatePostInput }, ctx) => {
  await ctx.restate.callObject("Post", args.id, "update_content", {
    title: args.input.title,
    description_markdown: args.input.descriptionMarkdown,
    summary: args.input.summary,
    post_type: args.input.postType,
    weight: args.input.weight,
    priority: args.input.priority,
    urgency: args.input.urgency,
    location: args.input.location,
  });
  ctx.loaders.postById.clear(args.id);
  return ctx.loaders.postById.load(args.id);
},
```

Update the `posts` query resolver to pass `submissionType`:

```typescript
posts: async (_parent, args, ctx) => {
  return ctx.restate.callService("Posts", "list", {
    ...existing fields...,
    submission_type: args.submissionType,  // NEW
  });
},
```

---

## Frontend Changes

### Install Plate.js

```bash
cd packages/admin-app && yarn add platejs @platejs/markdown @platejs/basic-marks @platejs/basic-blocks @platejs/link @platejs/image
```

See [CMS_EXPERIENCE.md](CMS_EXPERIENCE.md) Decision 1 for the full plugin list and shadcn component setup.

### New component: `packages/admin-app/components/admin/PostForm.tsx`

Shared form component used by both create and edit pages:

- **Fields:**
  - Title (text input)
  - Description (Plate.js — WYSIWYG markdown)
  - Summary (textarea, optional)
  - Post Type (select: story/notice/exchange/event/spotlight/reference)
  - Weight (select: heavy/medium/light)
  - Priority (number input)
  - Urgency (select: low/medium/high/urgent, optional)
  - Location (text input, optional)
  - Organization (select from OrganizationsListQuery, optional)

- **Props:**
  - `initialValues?: PostFormValues` — pre-filled for edit mode
  - `onSubmit: (values: PostFormValues) => Promise<void>`
  - `onCancel?: () => void`
  - `loading?: boolean`

- **Reuses:** `Input`, `Button`, `Card` from `components/ui/`, `OrganizationsListQuery` for org dropdown

### New page: `packages/admin-app/app/admin/(app)/posts/new/page.tsx`

- Uses `PostForm` with empty initial values
- Calls `CreatePostMutation` on submit
- Redirects to `/admin/posts/[id]` on success
- "Back to Posts" link at top

### Modified page: `packages/admin-app/app/admin/(app)/posts/[id]/page.tsx`

- Add "Edit" button to the header actions row
- When `isEditing` state is true, replace read-only content sections with `PostForm` pre-filled from post data
- "Save" calls `UpdatePostMutation`, then exits edit mode
- "Cancel" reverts to read-only view
- Existing approve/reject/archive actions remain available in read-only mode

### New queries: `packages/admin-app/lib/graphql/posts.ts`

```typescript
export const CreatePostMutation = graphql(`
  mutation CreatePost($input: CreatePostInput!) {
    createPost(input: $input) {
      id
      title
      status
    }
  }
`);

export const UpdatePostMutation = graphql(`
  mutation UpdatePost($id: ID!, $input: UpdatePostInput!) {
    updatePost(id: $id, input: $input) {
      id
      title
      status
      descriptionMarkdown
    }
  }
`);
```

### Sidebar: `packages/admin-app/components/admin/AdminSidebar.tsx`

No sidebar changes needed — the "Posts" link already exists. Add a "+ New" button on the posts list page header instead.

### Posts list: `packages/admin-app/app/admin/(app)/posts/page.tsx`

Add a "New Post" button in the page header that links to `/admin/posts/new`.

---

## Existing Code to Reuse

| What | Where | How |
|------|-------|-----|
| `CreatePost` builder | `post.rs:352` | Extend with `description_markdown` field |
| `UpdatePostContent` builder | `post.rs:394` | Already has all needed fields |
| `Post::update_content` SQL | `post.rs:933` | Already handles COALESCE for all content fields |
| `PostResult` response type | `virtual_objects/post.rs:211` | Already includes `description_markdown`, `submission_type` |
| `EditApproveRequest` pattern | `virtual_objects/post.rs:46` | Reference for content-update handler design |
| `PostDetailFields` fragment | `admin-app/lib/graphql/fragments.ts` | For loading edit form initial values |
| `ReactMarkdown` rendering | `posts/[id]/page.tsx` | For editor preview |
| `ui/Input`, `ui/Button`, `ui/Card` | `admin-app/components/ui/` | Form field components |
| `OrganizationsListQuery` | `admin-app/lib/graphql/organizations.ts` | For org selector dropdown |
| Resolver `callObject` pattern | `resolvers/post.ts:120` | `approvePost` is the template for `updatePost` |

---

## Implementation Steps

1. **Model**: Add `description_markdown` to `CreatePost` builder and `Post::create` SQL
2. **Model**: Add `post_type` and `submission_type` to `PostFilters`; update `find_paginated`, `find_paginated_near_zip`, `count_by_status` queries
3. **Activity**: Add `admin_create_post` and `admin_update_post` to `activities/core.rs`
4. **Activity**: Add `strip_markdown` helper (or add `pulldown-cmark` dependency)
5. **Restate**: Add `CreatePostRequest` + `create_post` handler to `PostsService`
6. **Restate**: Add `submission_type` field to `ListPostsRequest`; thread through to `PostFilters`
7. **Restate**: Add `UpdatePostContentRequest` + `update_content` handler to `Post` virtual object
8. **GraphQL**: Add input types, mutations, `submissionType` field to `schema.ts`
9. **GraphQL**: Add `createPost`, `updatePost` resolvers; update `posts` resolver
10. **Frontend**: Install Plate.js and plugins (see above)
11. **Frontend**: Build `PostForm` component
12. **Frontend**: Build `/admin/posts/new` page
13. **Frontend**: Add edit mode to `/admin/posts/[id]` page
14. **Frontend**: Add "New Post" button to posts list page
15. **Codegen**: Run `yarn codegen` in admin-app
16. **Rebuild**: `docker compose up -d --build server`

---

## Verification

1. Navigate to `/admin/posts`, click "New Post"
2. Fill out all form fields, type markdown in the editor
3. Submit — post should appear in the posts list with status `active`
4. Open the new post — markdown should render correctly
5. Click "Edit" — form should pre-fill with all post data
6. Modify title and markdown content, save
7. Verify changes persist on reload
8. Verify plain-text `description` was auto-generated from markdown
