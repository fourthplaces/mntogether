---
status: pending
priority: p1
issue_id: "003"
tags: [code-review, architecture, circular-dependency, refactoring]
dependencies: []
---

# Kernel Layer Depends on Domain Layer (Circular Dependency)

## Problem Statement

The kernel layer (infrastructure) imports types from the domain layer, creating a circular dependency that violates the dependency inversion principle. This makes the kernel non-reusable across projects and couples infrastructure to business logic.

## Findings

**Location**: `/packages/server/src/kernel/mod.rs:24`

**Violation**:
```rust
// VIOLATION: Kernel importing from domain
pub use crate::domains::organization::effects::need_extraction::{ContactInfo, ExtractedNeed};
```

**Dependency Graph**:
```
Kernel → Domain → Kernel (via traits)
  ↓        ↓
ExtractedNeed, ContactInfo
```

**From Architecture Strategist Agent**: "This creates a circular dependency violating the dependency inversion principle, making kernel not reusable across projects, coupling infrastructure to business logic, and preventing true separation of concerns."

**Impact**:
- Kernel cannot be extracted as reusable library
- Infrastructure tied to specific business domain
- Violates clean architecture principles
- Makes testing more complex

## Proposed Solutions

### Option 1: Move Types to Common Module (Recommended)
**Pros**: Clean separation, types accessible to both layers
**Cons**: New module to maintain
**Effort**: Small (30 minutes)
**Risk**: Low

```rust
// Create src/common/types.rs
pub struct ExtractedNeed {
    pub title: String,
    pub tldr: String,
    pub description: String,
    pub contact: Option<ContactInfo>,
    pub urgency: Option<String>,
    pub location: Option<String>,
}

pub struct ContactInfo {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub website: Option<String>,
}

// kernel/mod.rs - NO domain imports
pub use crate::common::types::{ExtractedNeed, ContactInfo};

// domain uses common types
use crate::common::types::{ExtractedNeed, ContactInfo};
```

### Option 2: Keep in Domain, Remove Kernel Re-export
**Pros**: Minimal changes, types stay with domain logic
**Cons**: Kernel users must import from domain (acceptable)
**Effort**: Small (15 minutes)
**Risk**: Low

```rust
// kernel/mod.rs - Remove this line:
// pub use crate::domains::organization::effects::need_extraction::...

// Domain functions return domain types
// Callers import from domain layer directly
use crate::domains::organization::effects::need_extraction::ExtractedNeed;
```

### Option 3: Use Generic Types in Kernel
**Pros**: Maximum flexibility, kernel truly generic
**Cons**: More complex, requires trait bounds
**Effort**: Large (3 hours)
**Risk**: Medium

```rust
// kernel/traits.rs
pub trait BaseAI: Send + Sync {
    async fn complete<T: DeserializeOwned>(&self, prompt: &str) -> Result<T>;
}

// Domain defines its own types
// No kernel dependency on domain types
```

## Recommended Action

**Option 1** - Create `src/common/types.rs` for cross-cutting types. This is the cleanest architectural solution and follows common Rust patterns.

## Technical Details

**Affected Files**:
- `/packages/server/src/kernel/mod.rs` (Line 24) - Remove domain import
- Create `/packages/server/src/common/types.rs` - New file
- `/packages/server/src/common/mod.rs` - New module declaration
- `/packages/server/src/lib.rs` - Add `pub mod common;`
- `/packages/server/src/domains/organization/effects/need_extraction.rs` - Update imports

**Module Structure After Fix**:
```
src/
├── common/
│   ├── mod.rs
│   └── types.rs (ExtractedNeed, ContactInfo)
├── kernel/
│   ├── mod.rs (no domain imports)
│   └── traits.rs
├── domains/
│   └── organization/
       └── effects/
           └── need_extraction.rs (uses common::types)
```

## Acceptance Criteria

- [ ] `ExtractedNeed` and `ContactInfo` moved to `common/types.rs`
- [ ] Kernel layer imports zero types from domain layer
- [ ] All domain code updated to use `common::types`
- [ ] All compilation errors fixed
- [ ] Tests pass
- [ ] Dependency graph verified: `Kernel ← Common → Domain`
- [ ] Architecture documentation updated

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/src/kernel/mod.rs:24` (violation)
  - `/packages/server/src/domains/organization/effects/need_extraction.rs:18-33` (type definitions)
- **Documentation**:
  - [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
  - [Dependency Inversion Principle](https://en.wikipedia.org/wiki/Dependency_inversion_principle)
- **Similar Patterns**: Common/shared types module is standard in large Rust projects
