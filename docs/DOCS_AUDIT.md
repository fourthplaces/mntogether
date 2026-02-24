# Documentation Audit — Root Editorial Pivot

**Date:** 2026-02-24
**Purpose:** Triage every doc in the repo against the pivot. What gets deleted, what stays, what needs editing.

---

## Summary

| Category | Count | Action |
|----------|-------|--------|
| **A: DELETE** | 33 files | Remove — dead systems, stale plans, irrelevant brainstorms |
| **B: KEEP AS-IS** | 38 files | No changes needed |
| **C: KEEP WITH EDITS** | 12 files | Contains valuable knowledge but references dead systems |

---

## A. DELETE (33 files)

These are entirely about dead concerns — crawling, extraction, Seesaw, chat/real-time, social media scraping, volunteer matching, or resolved migration blockers.

### docs/architecture/

| File | Why |
|------|-----|
| CURATOR_PIPELINE.md | AI curator for crawled web pages — Root Signal concern |
| CHAT_ARCHITECTURE.md | Real-time chat with Redis pub/sub — feature removed |
| COMPONENT_INVENTORY.md | Old "Minnesota Digital Aid" mobile/admin-spa components — completely replaced |
| domain-approval-workflow.md | Website/domain approval for crawling — Root Signal concern |

### docs/plans/

| File | Why |
|------|-----|
| 2026-01-27-mvp-execution-plan.md | Original "Emergency Resource Aggregator" MVP — pivoted away |
| 2026-01-29-feat-multi-sided-resource-platform-plan.md | Healthcare referral marketplace — dead concept |
| 2026-01-29-feat-referral-coordination-system-plan.md | Four-sided volunteer marketplace — dead |
| 2026-01-31-feat-agent-chat-with-machines-plan.md | Seesaw-based real-time agent chat — both dead |
| 2026-02-01-refactor-untangle-seesaw-architecture-plan.md | Seesaw 0.1.1 deep dive — framework removed |
| 2026-02-01-refactor-upgrade-seesaw-to-0.3.0-plan.md | Seesaw upgrade — framework removed |
| 2026-02-01-refactor-upgrade-seesaw-to-0.5.0-plan.md | Seesaw upgrade — framework removed |
| 2026-02-02-refactor-upgrade-seesaw-to-0.6.0-plan.md | Seesaw upgrade — framework removed |
| 2026-02-02-tavily-discovery-hybrid-crawl.md | Tavily web discovery — Root Signal concern |
| 2026-02-03-consolidate-crawling-to-extraction-library-plan.md | Crawling consolidation — Root Signal concern |
| 2026-02-03-design-extraction-library-plan.md | Extraction library design — Root Signal concern |
| 2026-02-03-refactor-extraction-system-alignment-plan.md | Dual extraction system alignment — both dead |
| 2026-02-04-feat-replace-nextjs-with-dioxus-fullstack-plan.md | Replace Next.js with Dioxus — abandoned |
| 2026-02-04-refactor-crawling-cascade-event-chaining-plan.md | Crawling event chains — Root Signal concern |
| 2026-02-04-refactor-split-crawl-pipeline-into-jobs-plan.md | Crawl pipeline jobs — Root Signal concern |
| 2026-02-04-refactor-upgrade-seesaw-to-0.7.2-plan.md | Seesaw upgrade — framework removed |
| 2026-02-05-refactor-upgrade-seesaw-to-0.8.0-plan.md | Seesaw upgrade — framework removed |
| 2026-02-10-feat-organizations-social-media-sources-plan.md | Social media scraping — Root Signal concern |

### docs/ (top-level)

| File | Why |
|------|-----|
| RESTATE_MIGRATION_BLOCKERS.md | Migration blocker that was resolved — no longer relevant |
| SNAPSHOT_TRACEABILITY.md | Crawling/extraction data flow tracing — Root Signal concern |

### docs/guides/

| File | Why |
|------|-----|
| DESIGNER_GUIDE.md | Old Expo mobile app + admin-spa styling — completely replaced |
| MATCHING_IMPLEMENTATION.md | Location-based volunteer matching — dead feature |

### docs/brainstorms/

| File | Why |
|------|-----|
| 2026-02-08-agents-brainstorm.md | Crawling/extraction agent pipelines — Root Signal concern |
| 2026-02-08-home-footer-actions-brainstorm.md | Old platform home/footer UX — superseded by broadsheet design |
| 2026-02-10-auto-create-organizations-brainstorm.md | Auto-create orgs from crawled websites — Root Signal concern |
| 2026-02-11-clean-up-org-posts-brainstorm.md | Crawling/sync cleanup — Root Signal concern |
| 2026-02-11-focused-post-extraction-brainstorm.md | Per-org extraction pipelines — Root Signal concern |

### docs/other

| File | Why |
|------|-----|
| 2026-02-04-refactor-split-crawl-pipeline-into-jobs-plan.md (top-level) | Crawl pipeline refactor — Root Signal concern |

---

## B. KEEP AS-IS (38 files)

These are still accurate and relevant. No edits needed.

### docs/architecture/

| File | What It Covers |
|------|---------------|
| TAGS_VS_FIELDS.md | Core principle: tags for discovery, fields for queries |
| DOMAIN_ARCHITECTURE.md | Layered domain structure (models, data, activities, restate) |
| PII_SCRUBBING.md | PII detection architecture — still needed |
| DATABASE_SCHEMA.md | Canonical schema reference (through migration 171) |
| SIMPLIFIED_SCHEMA.md | Minimal schema design philosophy |
| SCHEMA_DESIGN.md | Extension table patterns (base + business_organizations) |
| SCHEMA_RELATIONSHIPS.md | ER diagrams and query patterns |
| PACKAGE_STRUCTURE.md | Monorepo package layout |
| RUST_PROJECT_STRUCTURE.md | Rust project layout, binaries, kernel, domains |
| DESIGN_TOKENS.md | UI color/typography/spacing tokens |
| CAUSE_COMMERCE_ARCHITECTURE.md | Business listing design patterns |

### docs/plans/

| File | What It Covers |
|------|---------------|
| 2026-02-01-refactor-architectural-audit-cleanup-plan.md | SOLID violations analysis — patterns still relevant |
| 2026-02-02-fix-admin-authorization-security-gaps-plan.md | Auth security fixes — still applies to CMS |
| 2026-02-02-refactor-codebase-health-audit-plan.md | Codebase health findings |
| 2026-02-05-refactor-data-model-alignment-with-hsds-plan.md | HSDS data model alignment for posts/orgs |
| 2026-02-10-feat-notes-model-attachable-alerts-plan.md | Notes model — living feature |
| 2026-02-13-feat-graphql-bff-layer-plan.md | GraphQL BFF layer — current architecture |
| 2026-02-13-refactor-separate-web-apps-plan.md | App split — current architecture |
| 2026-02-13-refactor-graphql-architecture-audit-plan.md | GraphQL cleanup — still applies |
| ARCHIVED-2026-01-27-comprehensive-plan.md | Historical reference (explicitly archived) |

### docs/admin/

| File | What It Covers |
|------|---------------|
| ADMIN_IDENTIFIERS_MIGRATION.md | Rename from ADMIN_EMAILS — completed, accurate |
| POST_ROTATION_SYSTEM.md | Fair visibility algorithm for posts |
| TWILIO_EMAIL_IMPLEMENTATION.md | Email verification via Twilio |

### docs/ (top-level)

| File | What It Covers |
|------|---------------|
| ROOT_EDITORIAL_PIVOT.md | The pivot bible (this session) |
| RESTATE_MIGRATION_SUMMARY.md | Completed Restate migration — valid reference |
| TESTING_WORKFLOWS.md | Restate workflow testing guide |
| NEWSLETTER_INGESTION_RESEARCH.md | Newsletter source type research |

### docs/guides/

| File | What It Covers |
|------|---------------|
| EMBEDDED_FRONTENDS.md | Frontends as separate services — still accurate |

### docs/migrations/

| File | What It Covers |
|------|---------------|
| MIGRATION_CLAUDE_VOYAGE.md | AI provider migration history |
| YARN_MODERN_UPGRADE.md | Yarn 1 → Yarn 4 upgrade |

### docs/brainstorms/

| File | What It Covers |
|------|---------------|
| 2026-02-10-ai-note-generation-pipeline-brainstorm.md | AI-powered editorial notes — living feature |
| 2026-02-10-notes-model-brainstorm.md | Notes model design — living feature |
| 2026-02-10-unified-sources-brainstorm.md | Unified sources architecture |
| 2026-02-11-heat-map-brainstorm.md | Geographic visualization |
| 2026-02-11-schedule-aware-post-filtering-brainstorm.md | Publication scheduling — relevant to editions |
| 2026-02-11-wire-up-openrouter-v3-speciale-brainstorm.md | OpenRouter integration |
| 2026-02-11-zip-code-proximity-filtering-brainstorm.md | Geographic search |
| 2026-02-12-ai-consultant-brainstorm.md | AI story enhancement |
| 2026-02-13-newsletter-ingestion-brainstorm.md | Newsletter source type |
| 2026-02-13-separate-web-apps-brainstorm.md | App split architecture |

---

## C. KEEP WITH EDITS (12 files)

These contain structural knowledge worth preserving but reference dead systems. Edits described below.

### docs/architecture/

| File | Needed Edits |
|------|-------------|
| **DATA_MODEL.md** | Remove: volunteer intake, push notifications, discovery_queries, matching. Keep: posts, orgs, tags, locations, notes. Update "posts" description from "opportunity notifications" to "editorial stories." |
| **RUST_IMPLEMENTATION.md** | Update "Core Philosophy" from "relevance notifier" to "CMS for community journalism." Remove curator pipeline phases. Remove Firecrawl/Tavily/Apify from tech stack. Keep: Restate, ServerDeps, domain architecture. |

### docs/plans/

Most Category C plans in `/docs/plans/` can actually just be deleted rather than maintained — the effort to update 26 old plans isn't worth it. The valuable architectural patterns from them are captured in the architecture docs. **Recommendation: delete all Category C plans and keep only the 9 KEEP AS-IS plans.**

### docs/ (top-level)

| File | Needed Edits |
|------|-------------|
| **README.md** | Update title from "Minnesota Digital Aid" to "Root Editorial." Reorganize links into "Active" and "Historical." Add pivot context at top. |
| **INSTITUTIONAL_LEARNINGS.md** | Keep "Effects Must Be Thin" and separation of concerns. Remove: Seesaw comparisons, crawling domain examples, matching architecture. |
| **DOCKER_GUIDE.md** | Remove Firecrawl/Tavily/Voyage AI env vars. Update port numbers. Remove NATS service. Update service descriptions for CMS context. |
| **LOCAL_DEV_SETUP.md** | Update context from "Community Resource Platform" to CMS. Keep tag system and post/org relationship explanations. |
| **OPENAI_CLIENT_REFACTOR.md** | Update to reflect current ai-client package (Claude + OpenRouter). |

### docs/admin/

| File | Needed Edits |
|------|-------------|
| **ADMIN_QUICK_START.md** | Update terminology ("admin identifiers" not "admin emails"). Update admin actions context. |
| **ADMIN_EMAIL_SETUP.md** | Remove old mutation references (approveNeed, etc.). Add current CMS mutations. |

### docs/guides/

| File | Needed Edits |
|------|-------------|
| **API_INTEGRATION_GUIDE.md** | Update ports, architecture description, project overview for CMS. |
| **OPENROUTER_INTEGRATION.md** | Simplify from research doc to usage guide. Update code examples. |

### docs/migrations/

| File | Needed Edits |
|------|-------------|
| **SQL_QUERY_REFACTORING.md** | Remove matching domain sections. Keep posts/org patterns. |
| **WEB_APP_MIGRATION.md** | Update context from volunteer platform to CMS. Note admin-app + web-app split. |

---

## Recommendation

1. **Delete the 33 Category A files** — no value preserved by keeping them
2. **Leave the 38 Category B files** untouched
3. **For the 12 Category C files** — edit the 5 high-impact ones (README, INSTITUTIONAL_LEARNINGS, DOCKER_GUIDE, DATA_MODEL, RUST_IMPLEMENTATION), consider deleting the rest if the editing effort isn't worth it
4. **For the 26 Category C plans** — just delete them. The architectural patterns they contain are better captured in the architecture docs. Old plans about dead features aren't useful even with edits.
