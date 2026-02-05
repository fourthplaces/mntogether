---
title: "Introduce Builder Pattern for Functions with 5+ Parameters"
type: refactor
date: 2026-02-04
---

# Introduce Builder Pattern for Functions with 5+ Parameters

## Overview

Replace long positional parameter lists with `typed_builder` structs throughout the codebase. This eliminates noisy `None` values and makes code more readable at call sites.

**Before:**
```rust
Post::create(
    org_name,
    title,
    description,
    None, // tldr
    "opportunity".to_string(),
    "general".to_string(),
    None, // capacity_status
    None, // urgency
    None, // location
    "pending".to_string(),
    None, // source_language
    "user_submitted".to_string(),
    None, // submitted_by_admin_id
    None, // website_id
    None, // source_url
    None, // organization_id
    None, // revision_of_post_id
    &pool,
).await?
```

**After:**
```rust
Post::create(
    CreatePost::builder()
        .organization_name(org_name)
        .title(title)
        .description(description)
        .build(),
    &pool,
).await?
```

## Problem Statement

The codebase has many functions with 5-17 parameters. This causes:
1. **Noisy call sites** - long chains of `None` values obscure the meaningful arguments
2. **Positional errors** - easy to swap similarly-typed parameters
3. **Poor discoverability** - hard to know what options are available
4. **Maintenance burden** - adding a new parameter requires updating all call sites

## Proposed Solution

Use `typed_builder` crate (already a dependency) to create builder structs for all functions with 5+ parameters. Follow the existing `Job` builder pattern which uses `setter(into)` for ergonomic string handling.

## Technical Approach

### Canonical Builder Pattern

```rust
use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
#[builder(field_defaults(setter(into)))]  // Allows .title("foo") without .to_string()
pub struct CreatePost {
    // Required fields - no default
    pub organization_name: String,
    pub title: String,
    pub description: String,

    // Optional fields - have defaults
    #[builder(default)]
    pub tldr: Option<String>,
    #[builder(default = "opportunity".to_string())]
    pub post_type: String,
    #[builder(default = "general".to_string())]
    pub category: String,
    #[builder(default)]
    pub capacity_status: Option<String>,
    #[builder(default)]
    pub urgency: Option<String>,
    #[builder(default)]
    pub location: Option<serde_json::Value>,
    #[builder(default = "pending".to_string())]
    pub status: String,
    #[builder(default)]
    pub source_language: Option<String>,
    #[builder(default = "user_submitted".to_string())]
    pub submission_type: String,
    #[builder(default)]
    pub submitted_by_admin_id: Option<Uuid>,
    #[builder(default)]
    pub website_id: Option<Uuid>,
    #[builder(default)]
    pub source_url: Option<String>,
    #[builder(default)]
    pub organization_id: Option<Uuid>,
    #[builder(default)]
    pub revision_of_post_id: Option<Uuid>,
}
```

### Execution Pattern

Keep the existing pattern where the model method takes the builder output and pool separately:

```rust
impl Post {
    pub async fn create(input: CreatePost, pool: &PgPool) -> Result<Self> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO posts (...) VALUES (...) RETURNING *"
        )
        .bind(input.organization_name)
        .bind(input.title)
        // ... etc
        .fetch_one(pool)
        .await
        .map_err(Into::into)
    }
}
```

## Acceptance Criteria

- [x] All functions with 5+ parameters have builder structs
- [x] All call sites updated to use builders
- [x] No `#[allow(clippy::too_many_arguments)]` annotations remain (except ServerDeps::new - infrastructure)
- [x] Tests pass with identical behavior
- [x] Builder structs use `setter(into)` for String fields

## Implementation Phases

### Phase 1: High-Impact Post Operations (17 + 12 + 9 params)

| Function | File | Params | Call Sites |
|----------|------|--------|------------|
| `Post::create` | `posts/models/post.rs:398` | 17 | 7 |
| `create_post` | `posts/effects/post_operations.rs:16` | 12 | 1 |
| `Post::update_content` | `posts/models/post.rs:504` | 9 | 3 |

**Builder Structs:**
- `CreatePost` (for Post::create)
- `UpdatePostContent` (for Post::update_content)

**Files to modify:**

```
packages/server/src/domains/posts/models/post.rs
├── Add CreatePost builder struct
├── Add UpdatePostContent builder struct
├── Refactor Post::create to accept CreatePost
└── Refactor Post::update_content to accept UpdatePostContent

packages/server/src/domains/posts/effects/post_operations.rs
├── Update create_post to build CreatePost
└── Update update_and_approve_post to build UpdatePostContent

packages/server/src/domains/posts/effects/utils/sync_utils.rs
├── Update Post::create call site
└── Update Post::update_content call site

packages/server/src/domains/posts/effects/post.rs
└── Update Post::create call site (resource link handler)

packages/server/src/domains/posts/actions/llm_sync.rs
└── Update Post::create call site (revision creation)

packages/server/src/domains/posts/actions/create_post.rs
└── Update Post::create call site (extracted post creation)

packages/server/tests/common/fixtures.rs
├── Update test fixture Post::create calls (2 sites)
└── Consider test helper: CreatePost::builder().test_defaults()
```

### Phase 2: Assessment and Organization Operations (10 + 9 params)

| Function | File | Params | Call Sites |
|----------|------|--------|------------|
| `WebsiteAssessment::create` | `website/models/website_assessment.rs:53` | 10 | 1 |
| `Organization::update` | `organization/models/organization.rs:276` | 9 | 1 |

**Builder Structs:**
- `CreateWebsiteAssessment`
- `UpdateOrganization`

**Files to modify:**

```
packages/server/src/domains/website/models/website_assessment.rs
├── Add CreateWebsiteAssessment builder struct
└── Refactor WebsiteAssessment::create

packages/server/src/domains/website_approval/actions/mod.rs
└── Update generate_assessment call site

packages/server/src/domains/organization/models/organization.rs
├── Add UpdateOrganization builder struct (CreateOrganization already exists)
└── Refactor Organization::update
```

### Phase 3: Medium-Priority Functions (6-8 params)

| Function | File | Params | Call Sites |
|----------|------|--------|------------|
| `update_and_approve_post` | `post_operations.rs:91` | 9 | Internal |
| `add_provider_contact` | `providers/actions/mutations.rs:191` | 7 | 1 |
| `TavilySearchQuery::create` | `website/models/website_research.rs:115` | 6 | 1 |
| `PostContact::create` | `posts/models/post_contact.rs:44` | 6 | 1 |
| `Website::create` | `website/models/website.rs:194` | 6 | 3 |

**Builder Structs:**
- `UpdateAndApprovePost`
- `AddProviderContact`
- `CreateTavilySearchQuery`
- `CreatePostContact`
- `CreateWebsite`

### Phase 4: Lower-Priority Functions (5 params)

| Function | File | Params |
|----------|------|--------|
| `ingest_website` | `crawling/actions/ingest_website.rs:41` | 5 |
| `ingest_urls` | `crawling/actions/ingest_website.rs:165` | 5 |
| `register_member` | `member/actions/register_member.rs:15` | 5 |
| `generate_assessment` | `website_approval/actions/mod.rs:257` | 5 |
| `edit_and_approve_post` | `posts/actions/core.rs:108` | 5 |

**Builder Structs:**
- `IngestWebsite`
- `IngestUrls`
- `RegisterMember`
- `GenerateAssessment`
- `EditAndApprovePost`

## Field Mapping: CreatePost

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `organization_name` | String | ✅ Yes | - |
| `title` | String | ✅ Yes | - |
| `description` | String | ✅ Yes | - |
| `tldr` | Option<String> | No | None |
| `post_type` | String | No | "opportunity" |
| `category` | String | No | "general" |
| `capacity_status` | Option<String> | No | None |
| `urgency` | Option<String> | No | None |
| `location` | Option<Value> | No | None |
| `status` | String | No | "pending" |
| `source_language` | Option<String> | No | None |
| `submission_type` | String | No | "user_submitted" |
| `submitted_by_admin_id` | Option<Uuid> | No | None |
| `website_id` | Option<Uuid> | No | None |
| `source_url` | Option<String> | No | None |
| `organization_id` | Option<Uuid> | No | None |
| `revision_of_post_id` | Option<Uuid> | No | None |

## Test Strategy

1. **Migration tests** - Verify builder-based creation produces identical database rows
2. **Default value tests** - Confirm `#[builder(default)]` produces expected values
3. **Compile-time checks** - `typed_builder` enforces required fields at compile time

## Success Metrics

- All `#[allow(clippy::too_many_arguments)]` removed
- Reduced average call-site line count by 50%+
- No behavioral changes (pure refactor)

## Dependencies & Risks

**Dependencies:**
- `typed_builder` crate (already in Cargo.toml)

**Risks:**
- None identified - this is internal API only, no external consumers

## References

### Existing Builder Patterns in Codebase
- `packages/server/src/domains/organization/models/organization.rs:10-29` - CreateOrganization
- `packages/server/src/kernel/jobs/job.rs:346-439` - Job (with `setter(into)`)

### Files with `#[allow(clippy::too_many_arguments)]`
- `packages/server/src/domains/posts/models/post.rs`
- `packages/server/src/domains/website/models/website_assessment.rs`
