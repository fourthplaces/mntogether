# Root Editorial Documentation

Documentation for the Root Editorial CMS platform — an open CMS for community journalism.

> **See [ROOT_EDITORIAL_PIVOT.md](architecture/ROOT_EDITORIAL_PIVOT.md)** for the comprehensive pivot reference.

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
- [Package Structure](architecture/PACKAGE_STRUCTURE.md) - Monorepo organization
- [Rust Implementation](architecture/RUST_IMPLEMENTATION.md) - Rust backend details
- [Rust Project Structure](architecture/RUST_PROJECT_STRUCTURE.md) - Rust codebase layout
- [Database Schema](architecture/DATABASE_SCHEMA.md) - Canonical schema reference
- [Simplified Schema](architecture/SIMPLIFIED_SCHEMA.md) - Minimal schema philosophy
- [Tags vs Fields](architecture/TAGS_VS_FIELDS.md) - Data modeling decisions
- [Architecture Decisions](architecture/ARCHITECTURE_DECISIONS.md) - Tech stack slimming decisions (static site, Restate bypass, webhook integration)
- [Design Tokens](architecture/DESIGN_TOKENS.md) - Design system tokens
- [PII Scrubbing](architecture/PII_SCRUBBING.md) - PII detection architecture
- [Abuse Reporting](architecture/ABUSE_REPORTING.md) - Post reporting feature spec and current state

### Development Guides
- [API Integration Guide](guides/API_INTEGRATION_GUIDE.md) - Working with the GraphQL API
- [Institutional Learnings](guides/INSTITUTIONAL_LEARNINGS.md) - Hard-won lessons and gotchas
- [Testing Workflows](guides/TESTING_WORKFLOWS.md) - Restate workflow testing guide

### Admin & CMS
- [Admin Quick Start](admin/ADMIN_QUICK_START.md) - Admin/CMS authentication setup
- [Admin Email Setup](admin/ADMIN_EMAIL_SETUP.md) - Admin email configuration
- [Admin Identifiers Migration](admin/ADMIN_IDENTIFIERS_MIGRATION.md) - Auth identifier system
- [Post Rotation System](admin/POST_ROTATION_SYSTEM.md) - Content rotation logic
- [Twilio Email Implementation](admin/TWILIO_EMAIL_IMPLEMENTATION.md) - Twilio email integration

### Security & Authentication
- [Security](security/SECURITY.md) - Security overview and best practices
- [Authentication Guide](security/AUTHENTICATION_GUIDE.md) - Auth implementation guide
- [Authentication Security](security/AUTHENTICATION_SECURITY.md) - Auth security details

### Status & Postmortems
- [Phase 1: Dead Code Removal](status/PHASE_1_DEAD_CODE_REMOVAL.md)
- [Phase 2: Post Types](status/PHASE_2_POST_TYPES.md)
- [Phase 3: Edition System](status/PHASE_3_EDITION_SYSTEM.md)
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
