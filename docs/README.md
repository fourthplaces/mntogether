# Root Editorial Documentation

Documentation for the Root Editorial CMS platform — an open CMS for community journalism.

> **See [ROOT_EDITORIAL_PIVOT.md](architecture/ROOT_EDITORIAL_PIVOT.md)** for the comprehensive pivot reference.

## [→ Outstanding Work (TODO)](TODO.md)

What's done, what's next, what's punted. Start here for prioritization.

## [→ Decisions Log](DECISIONS_LOG.md)

Architectural decisions and the reasoning behind them. Read this before re-evaluating settled questions.

---

## Documentation Structure

### Getting Started
- [Local Dev Setup](setup/LOCAL_DEV_SETUP.md) - Local development environment and test data
- [Docker Guide](setup/DOCKER_GUIDE.md) - Docker Compose setup and commands
- [Quick Start](setup/QUICK_START.md) - Fast getting started guide
- [Docker Setup](setup/DOCKER_SETUP.md) - Docker and containerization setup
- [Deployment](setup/DEPLOYMENT.md) - Production deployment guide

### Architecture
- [Root Editorial Pivot](architecture/ROOT_EDITORIAL_PIVOT.md) - The pivot bible: what stays, what's dead, what's next
- [CMS System Spec](architecture/CMS_SYSTEM_SPEC.md) - Comprehensive CMS and broadsheet spec
- [Post Type System](architecture/POST_TYPE_SYSTEM.md) - Post types, field groups, and templates
- [Domain Architecture](architecture/DOMAIN_ARCHITECTURE.md) - Domain-driven design structure
- [Backend Overview](architecture/backend-overview.md) - Axum HTTP server architecture
- [Package Structure](architecture/PACKAGE_STRUCTURE.md) - Monorepo organization
- [Rust Implementation](architecture/RUST_IMPLEMENTATION.md) - Rust backend details
- [Rust Project Structure](architecture/RUST_PROJECT_STRUCTURE.md) - Rust codebase layout
- [Database Schema](architecture/DATABASE_SCHEMA.md) - Canonical schema reference (⚠️ stale — covers through migration 171, schema now at 206)
- [Data Model](architecture/DATA_MODEL.md) - Core entity relationships
- [Simplified Schema](architecture/SIMPLIFIED_SCHEMA.md) - Minimal schema philosophy
- [Tags vs Fields](architecture/TAGS_VS_FIELDS.md) - Data modeling decisions
- [Architecture Decisions](architecture/ARCHITECTURE_DECISIONS.md) - Tech stack slimming decisions (static site, Restate bypass, webhook integration)
- [Root Signal Spec](architecture/ROOT_SIGNAL_SPEC.md) - Root Signal API contract (draft)
- [Docker Architecture](architecture/DOCKER_ARCHITECTURE.md) - Docker dev environment: volumes, build pipeline, and trade-offs
- [Design Tokens](architecture/DESIGN_TOKENS.md) - Design system tokens
- [PII Scrubbing](architecture/PII_SCRUBBING.md) - PII detection architecture
- [Embedding Features Reference](architecture/EMBEDDING_FEATURES_REFERENCE.md) - Archival: removed AI/embedding features catalog
- [Abuse Reporting](architecture/ABUSE_REPORTING.md) - Post reporting feature spec (deferred)
- [Map Page Plan](architecture/MAP_PAGE_PLAN.md) - MVP map page plan (deferred)

### Phase 4 Design Docs
- [CMS Experience](architecture/phase4/CMS_EXPERIENCE.md) - Overall CMS UX vision
- [Story Editor](architecture/phase4/STORY_EDITOR.md) - Plate.js WYSIWYG editor plan
- [Signal Inbox](architecture/phase4/SIGNAL_INBOX.md) - Root Signal content triage
- [Email Newsletter](architecture/phase4/EMAIL_NEWSLETTER.md) - Newsletter system design (deferred)
- [Broadsheet Layout Editor](architecture/phase4/BROADSHEET_LAYOUT_EDITOR.md) - Admin layout editor
- [Edition Cockpit](architecture/phase4/EDITION_COCKPIT.md) - Dashboard design
- [Edition Kanban](architecture/phase4/EDITION_KANBAN.md) - Kanban workflow board
- [Edition Status Model](architecture/phase4/EDITION_STATUS_MODEL.md) - Edition lifecycle states
- [Editorial Workflow Rework](architecture/phase4/EDITORIAL_WORKFLOW_REWORK.md) - Workflow orientation
- [Navigation Hierarchy](architecture/phase4/NAVIGATION_HIERARCHY.md) - Admin sidebar structure
- [Widget System](architecture/phase4/WIDGET_SYSTEM.md) - Widget domain design

### Development Guides
- [API Integration Guide](guides/API_INTEGRATION_GUIDE.md) - Working with the GraphQL API
- [Institutional Learnings](guides/INSTITUTIONAL_LEARNINGS.md) - Hard-won lessons and gotchas
- [Testing Workflows](guides/TESTING_WORKFLOWS.md) - Restate workflow testing guide

### Admin & CMS
- [Admin Quick Start](admin/ADMIN_QUICK_START.md) - Admin/CMS authentication setup
- [Admin Email Setup](admin/ADMIN_EMAIL_SETUP.md) - Admin email configuration
- [Admin Identifiers Migration](admin/ADMIN_IDENTIFIERS_MIGRATION.md) - Auth identifier system
- [Post Rotation System](admin/POST_ROTATION_SYSTEM.md) - Content rotation logic
- [Twilio Email Implementation](admin/TWILIO_EMAIL_IMPLEMENTATION.md) - Twilio email/SMS integration

### Security & Authentication
- [Security](security/SECURITY.md) - Security overview and best practices
- [Authentication Guide](security/AUTHENTICATION_GUIDE.md) - Auth implementation guide
- [Authentication Security](security/AUTHENTICATION_SECURITY.md) - Auth security details

### Status & Postmortems
- [Phase 1: Dead Code Removal](status/PHASE_1_DEAD_CODE_REMOVAL.md) ✅
- [Phase 2: Post Types](status/PHASE_2_POST_TYPES.md) ✅
- [Phase 3: Edition System](status/PHASE_3_EDITION_SYSTEM.md) ✅
- [Phase 4: CMS UX Rework](status/PHASE4_CMS_UX_REWORK.md) ✅ (frontend complete)
- [Broadsheet Design Import](status/BROADSHEET_DESIGN_IMPORT.md) ✅
- [Final Schema Summary](status/FINAL_SCHEMA_SUMMARY.md)
- [Turbopack CPU Loop Postmortem](status/TURBOPACK_CPU_LOOP_POSTMORTEM.md)
- [Docker Stale Dependencies Postmortem](status/DOCKER_STALE_DEPS_POSTMORTEM.md)

### Prompts
- [Prompts Index](prompts/README.md) - LLM prompts used in the codebase

## Related Resources

- [Main README](../README.md) - Project overview and quick start
- [Data README](../data/README.md) - Data directory documentation
- [Server Tests README](../packages/server/tests/README.md) - Test documentation

## Package Documentation

- [Admin App](../packages/admin-app/README.md) - Next.js CMS admin panel
- [Web App](../packages/web-app/README.md) - Next.js public web application
- [Shared](../packages/shared/README.md) - Shared GraphQL schema and types
- [Twilio RS](../packages/twilio-rs/README.md) - Twilio Rust library
