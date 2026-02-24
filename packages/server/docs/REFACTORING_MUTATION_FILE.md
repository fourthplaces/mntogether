# Refactoring Plan: Split mutation.rs File

## Current State

**File**: `src/domains/listings/edges/mutation.rs`
**Size**: 885 lines
**Issue**: Single large file containing all GraphQL mutations for listings, posts, and domains

## Proposed Structure

Split into 4 focused modules:

```
src/domains/listings/edges/
├── mod.rs                  # Re-exports all mutations
├── listing_mutations.rs    # Listing CRUD operations (~250 lines)
├── post_mutations.rs       # Post management (~200 lines)
├── domain_mutations.rs     # Domain/source operations (~300 lines)
└── resource_mutations.rs   # Resource link submissions (~135 lines)
```

## Function Distribution

### 1. listing_mutations.rs

**Functions**:
- `submit_listing()` - Create new listing from user submission
- `approve_listing()` - Approve pending listing
- `edit_and_approve_listing()` - Edit and approve in one operation
- `reject_listing()` - Reject listing with reason
- `delete_listing()` - Delete listing

**Common imports needed**:
```rust
use crate::common::{JobId, ListingId, MemberId};
use crate::domains::listings::data::{EditListingInput, ListingType, SubmitListingInput};
use crate::domains/listings::events::ListingEvent;
use crate::domains::listings::models::Listing;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::{dispatch_request, EnvelopeMatch};
use tracing::info;
use uuid::Uuid;
```

**Estimated size**: ~250 lines

### 2. post_mutations.rs

**Functions**:
- `repost_listing()` - Create new post for existing listing
- `expire_post()` - Mark post as expired
- `archive_post()` - Archive a post
- `track_post_view()` - Increment post view count
- `track_post_click()` - Increment post click count

**Common imports needed**:
```rust
use crate::common::{ListingId, MemberId, PostId};
use crate::domains::listings::events::ListingEvent;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::{dispatch_request, EnvelopeMatch};
use tracing::info;
use uuid::Uuid;
```

**Estimated size**: ~200 lines

### 3. domain_mutations.rs

**Functions**:
- `scrape_organization()` - Trigger source scraping
- `approve_domain()` - Approve pending domain
- `reject_domain()` - Reject domain with reason
- `suspend_domain()` - Suspend domain
- `refresh_page_snapshot()` - Refresh cached page content

**Common imports needed**:
```rust
use crate::common::{DomainId, JobId, MemberId};
use crate::domains::listings::data::ScrapeJobResult;
use crate::domains::listings::events::ListingEvent;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::{dispatch_request, EnvelopeMatch};
use tracing::info;
use uuid::Uuid;
```

**Estimated size**: ~300 lines

### 4. resource_mutations.rs

**Functions**:
- `submit_resource_link()` - Submit external resource link

**Common imports needed**:
```rust
use crate::common::JobId;
use crate::domains::listings::data::{SubmitResourceLinkInput, SubmitResourceLinkResult};
use crate::domains::listings::events::ListingEvent;
use crate::server::graphql::context::GraphQLContext;
use juniper::{FieldError, FieldResult};
use seesaw_core::{dispatch_request, EnvelopeMatch};
use tracing::info;
```

**Estimated size**: ~135 lines

## Step-by-Step Refactoring Process

### Step 1: Create listing_mutations.rs

1. Create new file: `src/domains/listings/edges/listing_mutations.rs`
2. Copy common imports from mutation.rs
3. Move these functions from mutation.rs:
   - `submit_listing()`
   - `approve_listing()`
   - `edit_and_approve_listing()`
   - `reject_listing()`
   - `delete_listing()`
4. Remove unused imports
5. Test compilation: `cargo check`

### Step 2: Create post_mutations.rs

1. Create new file: `src/domains/listings/edges/post_mutations.rs`
2. Copy common imports
3. Move these functions:
   - `repost_listing()`
   - `expire_post()`
   - `archive_post()`
   - `track_post_view()`
   - `track_post_click()`
4. Test compilation: `cargo check`

### Step 3: Create domain_mutations.rs

1. Create new file: `src/domains/listings/edges/domain_mutations.rs`
2. Copy common imports
3. Move these functions:
   - `scrape_organization()`
   - `approve_domain()`
   - `reject_domain()`
   - `suspend_domain()`
   - `refresh_page_snapshot()`
4. Test compilation: `cargo check`

### Step 4: Create resource_mutations.rs

1. Create new file: `src/domains/listings/edges/resource_mutations.rs`
2. Copy common imports
3. Move function:
   - `submit_resource_link()`
4. Test compilation: `cargo check`

### Step 5: Update mod.rs

Update `src/domains/listings/edges/mod.rs` to include and re-export all modules:

```rust
pub mod listing_mutations;
pub mod post_mutations;
pub mod domain_mutations;
pub mod resource_mutations;
pub mod query;
pub mod agent_mutation;
pub mod agent_queries;

// Re-export all mutation functions
pub use listing_mutations::*;
pub use post_mutations::*;
pub use domain_mutations::*;
pub use resource_mutations::*;
```

### Step 6: Delete Original mutation.rs

Once all functions are moved and tests pass:
1. Delete `src/domains/listings/edges/mutation.rs`
2. Remove `pub mod mutation;` from mod.rs
3. Final test: `cargo check && cargo test`

## Verification Checklist

After refactoring:

- [ ] All files compile without errors
- [ ] No unused import warnings
- [ ] GraphQL schema still generates correctly
- [ ] Integration tests pass
- [ ] Each file is under 350 lines
- [ ] Functions are logically grouped
- [ ] No duplicate imports or code

## Benefits

### Improved Maintainability
- Easier to find specific mutation implementations
- Clearer separation of concerns
- Reduced cognitive load when working on specific features

### Better Organization
- Listing operations grouped together
- Post operations grouped together
- Domain operations grouped together
- Resource operations isolated

### Scalability
- Easy to add new mutations to appropriate modules
- Clear pattern for future module additions
- Reduced merge conflicts (changes isolated to specific modules)

## Timeline Estimate

- **Mechanical refactoring**: 2-3 hours
- **Testing and verification**: 1 hour
- **Total**: 3-4 hours

## Alternative: Gradual Migration

If full refactoring is too disruptive, consider gradual migration:

1. Create new modules with empty stubs
2. Migrate one function at a time
3. Keep deprecated re-exports in mutation.rs
4. Remove mutation.rs once all functions migrated

```rust
// In mutation.rs during migration
#[deprecated(note = "Use listing_mutations::submit_listing instead")]
pub use listing_mutations::submit_listing;
```

## Notes

- This is a **mechanical refactoring** - no logic changes
- All function signatures remain identical
- GraphQL schema is unaffected
- Tests should pass without modification
- Consider doing this during low-activity period to minimize merge conflicts
