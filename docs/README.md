# Root Editorial Documentation

Documentation for the Root Editorial CMS platform — an open CMS for community journalism.

> **See [ROOT_EDITORIAL_PIVOT.md](architecture/ROOT_EDITORIAL_PIVOT.md)** for the comprehensive pivot reference.

## Documentation Structure

### Getting Started
- [Local Dev Setup](setup/LOCAL_DEV_SETUP.md) - Local development environment and test data
- [Docker Guide](setup/DOCKER_GUIDE.md) - Docker Compose setup and commands
- [Quick Start](setup/QUICK_START.md) - Fast getting started guide
- [Development CLI](setup/DEV_CLI.md) - Interactive development CLI usage
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
- [Schema Design](architecture/SCHEMA_DESIGN.md) - Extension table patterns
- [Schema Relationships](architecture/SCHEMA_RELATIONSHIPS.md) - ER diagrams and query patterns
- [Simplified Schema](architecture/SIMPLIFIED_SCHEMA.md) - Minimal schema philosophy
- [Tags vs Fields](architecture/TAGS_VS_FIELDS.md) - Data modeling decisions
- [Design Tokens](architecture/DESIGN_TOKENS.md) - Design system tokens
- [PII Scrubbing](architecture/PII_SCRUBBING.md) - PII detection architecture

### Development Guides
- [API Integration Guide](guides/API_INTEGRATION_GUIDE.md) - Working with the GraphQL API
- [Embedded Frontends](guides/EMBEDDED_FRONTENDS.md) - Frontend architecture patterns
- [OpenRouter Integration](guides/OPENROUTER_INTEGRATION.md) - AI service integration
- [Institutional Learnings](guides/INSTITUTIONAL_LEARNINGS.md) - Hard-won lessons and gotchas
- [Testing Workflows](guides/TESTING_WORKFLOWS.md) - Restate workflow testing guide

### Admin & CMS
- [Admin Quick Start](admin/ADMIN_QUICK_START.md) - Admin/CMS authentication setup
- [Admin Identifiers Migration](admin/ADMIN_IDENTIFIERS_MIGRATION.md) - Auth identifier system
- [Post Rotation System](admin/POST_ROTATION_SYSTEM.md) - Content rotation logic
- [Twilio Email Implementation](admin/TWILIO_EMAIL_IMPLEMENTATION.md) - Twilio email integration

### Security & Authentication
- [Security](security/SECURITY.md) - Security overview and best practices
- [Authentication Guide](security/AUTHENTICATION_GUIDE.md) - Auth implementation guide
- [Authentication Security](security/AUTHENTICATION_SECURITY.md) - Auth security details

### Migrations (Historical)
- [Yarn Modern Upgrade](migrations/YARN_MODERN_UPGRADE.md) - Yarn 1 → Yarn 4 upgrade
- [Web App Migration](migrations/WEB_APP_MIGRATION.md) - Frontend migration history
- [Claude/Voyage Migration](migrations/MIGRATION_CLAUDE_VOYAGE.md) - AI provider migration history

### Prompts
- [Prompts Index](prompts/README.md) - LLM prompts used in the codebase

### Plans
- [Plans Index](plans/README.md) - Architectural plans and brainstorms

## Related Resources

- [Main README](../README.md) - Project overview and quick start
- [Data README](../data/README.md) - Data directory documentation
- [Server Tests README](../packages/server/tests/README.md) - Test documentation

## Package Documentation

- [Admin App](../packages/admin-app/README.md) - Next.js CMS admin panel
- [Web App](../packages/web-app/README.md) - Next.js public web application
- [Shared](../packages/shared/README.md) - Shared GraphQL schema and types
- [Twilio RS](../packages/twilio-rs/README.md) - Twilio Rust library
