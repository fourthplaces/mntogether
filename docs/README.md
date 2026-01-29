# Minnesota Digital Aid Documentation

Complete documentation for the Minnesota Digital Aid platform.

## 📚 Documentation Structure

### Setup & Getting Started
Getting the project up and running on your machine.

- [Quick Start](setup/QUICK_START.md) - Fast getting started guide
- [Development CLI](setup/DEV_CLI.md) - Interactive development CLI usage
- [Development Setup Complete](setup/DEV_SETUP_COMPLETE.md) - Complete setup verification
- [Docker Setup](setup/DOCKER_SETUP.md) - Docker and containerization setup
- [Deployment](setup/DEPLOYMENT.md) - Production deployment guide

### Architecture
System design, data models, and architectural decisions.

- [Domain Architecture](architecture/DOMAIN_ARCHITECTURE.md) - Domain-driven design structure
- [Chat Architecture](architecture/CHAT_ARCHITECTURE.md) - Real-time chat system design
- [Seesaw Architecture](architecture/SEESAW_ARCHITECTURE.md) - Event-driven architecture
- [Package Structure](architecture/PACKAGE_STRUCTURE.md) - Monorepo organization
- [Rust Implementation](architecture/RUST_IMPLEMENTATION.md) - Rust backend details
- [Rust Project Structure](architecture/RUST_PROJECT_STRUCTURE.md) - Rust codebase layout
- [Cause Commerce Architecture](architecture/CAUSE_COMMERCE_ARCHITECTURE.md) - Business features
- [Component Inventory](architecture/COMPONENT_INVENTORY.md) - UI component catalog
- [Design Tokens](architecture/DESIGN_TOKENS.md) - Design system tokens
- [Schema Design](architecture/SCHEMA_DESIGN.md) - Database schema design
- [Schema Relationships](architecture/SCHEMA_RELATIONSHIPS.md) - Data model relationships
- [Simplified Schema](architecture/SIMPLIFIED_SCHEMA.md) - Streamlined schema overview
- [Tags vs Fields](architecture/TAGS_VS_FIELDS.md) - Data modeling decisions

### Development Guides
Guides for building and extending the platform.

- [API Integration Guide](guides/API_INTEGRATION_GUIDE.md) - Working with the GraphQL API
- [Designer Guide](guides/DESIGNER_GUIDE.md) - UI/UX design guidelines
- [Embedded Frontends](guides/EMBEDDED_FRONTENDS.md) - Frontend architecture patterns
- [GraphQL Integration](guides/GRAPHQL_INTEGRATION.md) - GraphQL setup and usage
- [Matching Implementation](guides/MATCHING_IMPLEMENTATION.md) - Volunteer matching algorithm
- [OpenRouter Integration](guides/OPENROUTER_INTEGRATION.md) - AI service integration

### Admin & Operations
Administrative functions and operational procedures.

- [Admin Quick Start](admin/ADMIN_QUICK_START.md) - Admin interface overview
- [Admin Email Setup](admin/ADMIN_EMAIL_SETUP.md) - Email configuration
- [Admin Identifiers Migration](admin/ADMIN_IDENTIFIERS_MIGRATION.md) - ID system changes
- [Post Rotation System](admin/POST_ROTATION_SYSTEM.md) - Content rotation logic
- [Twilio Email Implementation](admin/TWILIO_EMAIL_IMPLEMENTATION.md) - Twilio email integration

### Security & Authentication
Security practices and authentication systems.

- [Security](security/SECURITY.md) - Security overview and best practices
- [Authentication Guide](security/AUTHENTICATION_GUIDE.md) - Auth implementation guide
- [Authentication Security](security/AUTHENTICATION_SECURITY.md) - Auth security details

### Migrations
Migration guides and database/system changes.

- [Migration Claude Voyage](migrations/MIGRATION_CLAUDE_VOYAGE.md) - AI model migration
- [Web App Migration](migrations/WEB_APP_MIGRATION.md) - Frontend migration
- [Yarn Modern Upgrade](migrations/YARN_MODERN_UPGRADE.md) - Package manager upgrade
- [SQL Query Refactoring](migrations/SQL_QUERY_REFACTORING.md) - Database query improvements

### Status & Progress
Project status, milestones, and progress tracking.

- [Current Status](status/CURRENT_STATUS.md) - Current development status
- [Implementation Complete](status/IMPLEMENTATION_COMPLETE.md) - Completed implementations
- [Implementation Summary](status/IMPLEMENTATION_SUMMARY.md) - Implementation overview
- [Implementation Progress](status/IMPLEMENTATION_PROGRESS.md) - Development timeline
- [Consolidation Complete](status/CONSOLIDATION_COMPLETE.md) - Code consolidation status
- [Integration Complete](status/INTEGRATION_COMPLETE.md) - Integration milestones
- [MVP Complete](status/MVP_COMPLETE.md) - MVP achievement
- [Refactoring Complete](status/REFACTORING_COMPLETE.md) - Refactoring milestones
- [Progress Summary](status/PROGRESS_SUMMARY.md) - Overall progress
- [Final Schema Summary](status/FINAL_SCHEMA_SUMMARY.md) - Schema finalization
- [Changes Summary](status/CHANGES_SUMMARY.md) - Recent changes
- [Cost Analysis](status/COST_ANALYSIS.md) - Infrastructure cost analysis
- [Production Readiness](status/PRODUCTION_READINESS.md) - Production deployment readiness
- [Spike 1 Complete](status/SPIKE_1_COMPLETE.md) - First technical spike
- [Technical Spikes](status/TECHNICAL_SPIKES.md) - Technical investigation spikes
- [Problem Solution](status/PROBLEM_SOLUTION.md) - Problem-solution mapping
- [Need Synchronization](status/NEED_SYNCHRONIZATION.md) - Synchronization requirements
- [User Submitted Needs](status/USER_SUBMITTED_NEEDS.md) - User feature requests

### Planning
Project plans and roadmaps.

- [Plans](plans/README.md) - Project planning documents

## 🔗 Related Resources

- [Main README](../README.md) - Project overview and quick start
- [Data README](../data/README.md) - Data directory documentation
- [Infrastructure README](../infra/README.md) - Infrastructure setup
- [Server Tests README](../packages/server/tests/README.md) - Test documentation

## Package Documentation

- [Web App](../packages/web-app/README.md) - React Native mobile app
- [Web Next](../packages/web-next/README.md) - Next.js web application
- [Intelligent Crawler](../packages/intelligent-crawler/README.md) - Web scraping service
- [Dev CLI](../packages/dev-cli/README.md) - Development CLI tool
- [Twilio RS](../packages/twilio-rs/README.md) - Twilio Rust library
