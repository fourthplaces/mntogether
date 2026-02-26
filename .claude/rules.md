# Project Rules

## Documentation Organization

**All documentation files MUST be placed in the `docs/` directory, not in the project root.**

### Directory Structure

```
docs/
├── admin/           # Admin-specific guides and setup
├── architecture/    # System architecture and design documents
├── guides/          # Implementation guides, tutorials, and reference
├── migrations/      # Database and code migration guides
├── plans/           # Project plans, roadmaps, and brainstorms
├── prompts/         # LLM prompts used in the codebase
├── security/        # Security policies and authentication
├── setup/           # Setup and deployment instructions
└── status/          # Implementation status, postmortems, and progress reports
```

### File Placement Rules

- **Architecture docs** → `docs/architecture/`
  - Data models, system design, component structure
  - Examples: `DATA_MODEL.md`, `SCHEMA_DESIGN.md`, `DOMAIN_ARCHITECTURE.md`

- **Implementation status** → `docs/status/`
  - Completion reports, postmortems, progress summaries
  - Examples: `FINAL_SCHEMA_SUMMARY.md`, `PHASE_1_DEAD_CODE_REMOVAL.md`

- **Setup/deployment guides** → `docs/setup/`
  - Installation, configuration, deployment instructions
  - Examples: `QUICK_START.md`, `DOCKER_SETUP.md`

- **Admin guides** → `docs/admin/`
  - Admin-specific setup and configuration
  - Examples: `ADMIN_EMAIL_SETUP.md`, `POST_ROTATION_SYSTEM.md`

- **Integration guides** → `docs/guides/`
  - API integration, feature implementation guides, reference material
  - Examples: `API_INTEGRATION_GUIDE.md`, `INSTITUTIONAL_LEARNINGS.md`

- **Security docs** → `docs/security/`
  - Authentication, authorization, security policies
  - Examples: `AUTHENTICATION_GUIDE.md`, `SECURITY.md`

- **Migration docs** → `docs/migrations/`
  - Schema migrations, code refactoring summaries
  - Examples: `MIGRATION_CLAUDE_VOYAGE.md`, `SQL_QUERY_REFACTORING.md`

- **Plans** → `docs/plans/`
  - Project plans, roadmaps, brainstorms, spikes
  - Examples: `2026-02-13-feat-graphql-bff-layer-plan.md`

- **Prompts** → `docs/prompts/`
  - LLM prompts used in the codebase (system prompts, generation templates)
  - Examples: `posts/writing-style.md`, `pii/pii-detection.md`

### Exception

**`README.md` is the ONLY documentation file that belongs in the project root** as it's the standard entry point for the repository.

### When Creating Documentation

Before creating any `.md` file:
1. Determine which category it belongs to
2. Place it in the appropriate `docs/` subdirectory
3. Use clear, descriptive filenames in SCREAMING_SNAKE_CASE or kebab-case
4. Update `docs/README.md` with a link if it's a major document
