# Embedding & AI Features Reference

Removed March 2026. All AI/LLM work now handled externally by Root Signal. This document catalogues what existed, ranked by utility, for potential future re-enablement.

## What Was Removed

- **`packages/ai-client/`** — Standalone crate wrapping OpenAI (GPT-5, GPT-5-Mini), OpenRouter, and Claude APIs. Provided `extract<T>()` (structured JSON extraction), `complete()` (free-form completion), and `create_embedding()`.
- **`EmbeddingService`** — Called OpenAI `text-embedding-3-small` to produce 1024-dimensional vectors. Stored in PostgreSQL via `pgvector` extension.
- **`ServerDeps.ai`** / **`ServerDeps.embedding_service`** — Injected into all activities via the kernel dependency container.
- **Database columns** — `posts.embedding`, `posts.relevance_score`, `posts.relevance_breakdown`, `members.embedding`, `notes.embedding` (dropped in migration 000193).

## Features Ranked by Utility

### Tier 1 — High Value (re-enable first if needed)

**1. Semantic Search (admin)**
- What: Vector similarity search across posts. Admin types a query, system returns posts ranked by meaning rather than keyword match.
- How it worked: Query text → embedding → cosine similarity against `posts.embedding` → ranked results with optional location weighting.
- Files: `posts/activities/search.rs`, `Post::search_by_similarity()`, `Post::search_by_similarity_with_location()`.
- Re-enable path: Add embedding column back, generate embeddings on post create/update, add search endpoint. Consider using Root Signal for embedding generation instead of direct OpenAI calls.

**2. AI-Powered Summary Generation**
- What: Generated concise summaries of post descriptions using GPT.
- How it worked: Post description → GPT prompt → 1-2 sentence summary stored in `posts.summary`.
- Files: `posts/activities/post_extraction.rs::generate_summary()`.
- Current fallback: Simple text truncation to 250 chars (`common::utils::content::generate_summary`).
- Re-enable path: Call Root Signal or OpenAI for summary generation in `create_post()`. Low effort.

### Tier 2 — Medium Value

**3. Relevance Scoring**
- What: Scored each post's relevance to its organization's mission (0-100 scale with breakdown).
- How it worked: Post + org context → GPT structured extraction → score + text breakdown stored in `posts.relevance_score` and `posts.relevance_breakdown`.
- Files: `posts/activities/scoring.rs`, `Post::update_relevance_score()`, `Post::find_unscored_active()`.
- Re-enable path: Add columns back, batch scoring endpoint. Could be useful for editorial prioritization.

**4. AI Tag Classification**
- What: Auto-generated tags for posts using GPT structured extraction.
- How it worked: Post content → GPT with tag taxonomy schema → extracted tags applied to post.
- Files: `posts/activities/tags.rs::regenerate_post_tags()`, `ExtractedTags` struct.
- Current state: Manual tag management still works (`add_post_tag`, `remove_post_tag`, `update_post_tags`).
- Re-enable path: New activity calling Root Signal with tag taxonomy, wire to admin UI button.

**5. Note-to-Post Semantic Attachment**
- What: Automatically linked notes to relevant posts within an organization using embedding similarity.
- How it worked: Note embedding vs post embeddings → cosine similarity > threshold → create `noteable` link.
- Files: `notes/activities/attachment.rs::attach_notes_to_org_posts()`.
- Current state: Manual linking via admin UI (`linkNote` mutation).
- Re-enable path: Generate embeddings for both notes and posts, similarity match, create links. Medium effort.

### Tier 3 — Low Value / Speculative

**6. Post Content Extraction & Rewriting**
- What: Extracted structured data from raw post content (contacts, schedules, eligibility) and rewrote narratives.
- How it worked: Raw HTML/text → GPT structured extraction → typed structs (ContactInfo, ScheduleInfo, etc.). Also rewrote post descriptions into editorial style.
- Files: `posts/activities/post_extraction.rs` (extract, rewrite, summarize functions).
- Re-enable path: Root Signal likely handles this already. Only re-add if Root Signal doesn't cover structured extraction.

**7. Member Embedding**
- What: Generated embedding vectors for member profiles.
- How it worked: Member profile text → OpenAI embedding → stored in `members.embedding`.
- Files: `member/activities/generate_embedding.rs`, `Member::update_embedding()`.
- Usage: Was generated on registration but never queried. No feature consumed it.
- Re-enable path: Only if member-to-post matching or personalized feeds are needed.

**8. Public Semantic Search (web app)**
- What: Would have allowed public users to search posts by meaning.
- Status: Never shipped to production. Admin-only search existed.
- Re-enable path: Same as admin semantic search but with public endpoint + rate limiting.

## Technical Details for Re-enablement

### PostgreSQL pgvector Setup
The `pgvector` extension is already enabled in the database (migration 000017). To re-add embedding columns:

```sql
ALTER TABLE posts ADD COLUMN embedding vector(1024);
CREATE INDEX posts_embedding_idx ON posts USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
```

### Embedding Model
Used `text-embedding-3-small` (OpenAI) with 1024 dimensions. Good balance of quality and cost. Alternatives: `text-embedding-3-large` (3072 dims, higher quality), or open-source models via Root Signal.

### Cargo Dependencies
To re-add pgvector support to the Rust server:
```toml
pgvector = { version = "0.4", features = ["sqlx", "serde"] }
```

### Kernel Integration Pattern
If re-adding AI services, follow the existing kernel pattern:
1. Define trait in `kernel/traits.rs` (e.g., `BaseEmbeddingService`)
2. Implement in `common/utils/` or a dedicated crate
3. Add to `ServerDeps` struct in `kernel/deps.rs`
4. Inject via constructor in `bin/server.rs`
5. Mock in `kernel/test_dependencies.rs` for tests
