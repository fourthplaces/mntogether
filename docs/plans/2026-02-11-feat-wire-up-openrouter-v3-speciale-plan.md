---
title: "feat: Wire up OpenRouter with DeepSeek V3.2-Speciale"
type: feat
date: 2026-02-11
---

# feat: Wire up OpenRouter with DeepSeek V3.2-Speciale

## Overview

Add `ai_next: Arc<OpenRouter>` to ServerDeps and switch 19 accuracy-critical AI call sites from GPT-4o to DeepSeek V3.2-Speciale via OpenRouter. Keep GPT-4o for 5 hallucination-tolerant tasks and all embeddings.

**Cost impact**: V3.2-Speciale at $0.27/$0.41 per 1M tokens vs GPT-4o at $2.50/$10.00 — 9x/24x cheaper with higher reasoning accuracy.

## Proposed Solution

Two AI clients in ServerDeps with a centralized model constant:

```rust
// kernel/mod.rs
pub const FRONTIER_MODEL: &str = "deepseek/deepseek-v3.2-speciale";

// kernel/deps.rs
pub struct ServerDeps {
    pub ai: Arc<OpenAi>,          // GPT-4o — embeddings + Tier 2 tasks
    pub ai_next: Arc<OpenRouter>,  // V3.2-Speciale — Tier 1 tasks
    // ... rest unchanged
}
```

## Acceptance Criteria

- [x] `OPENROUTER_API_KEY` loaded in `server.rs`, required at startup
- [x] `ai_next: Arc<OpenRouter>` field added to `ServerDeps`
- [x] Model name centralized as `FRONTIER_MODEL` constant
- [x] 19 Tier 1 call sites use `deps.ai_next` / `ai_next.as_ref()`
- [x] 5 Tier 2 call sites unchanged on `deps.ai`
- [x] 5 embedding call sites unchanged on `deps.ai`
- [x] `TestDependencies` updated with `ai_next` field + mock
- [x] `.env.example` updated with `OPENROUTER_API_KEY`
- [x] `cargo check` passes
- [x] `cargo test -p server --lib` passes

## Implementation Tasks

### Phase 1: Infrastructure (ServerDeps + Config)

- [x] **Add `FRONTIER_MODEL` constant to `kernel/mod.rs`**
  - `pub const FRONTIER_MODEL: &str = "deepseek/deepseek-v3.2-speciale";`
  - Also add `pub use ai_client::OpenRouter;` re-export

- [x] **Add `ai_next` field to `kernel/deps.rs`**
  - Add `use ai_client::OpenRouter;`
  - Add field: `pub ai_next: Arc<OpenRouter>`
  - Update `ServerDeps::new()` constructor to accept and store it

- [x] **Wire up in `bin/server.rs`**
  - Load `OPENROUTER_API_KEY` env var (required — fail with clear message if missing)
  - Create `Arc<OpenRouter>` with model from `FRONTIER_MODEL` constant
  - Set `.with_app_name("MN Together")` and `.with_site_url("https://mntogether.org")`
  - Pass to `ServerDeps::new()`

- [x] **Update `kernel/test_dependencies.rs`**
  - Add `pub ai_next: Arc<OpenRouter>` field to `TestDependencies`
  - Create `mock_openrouter_client()`: `Arc::new(OpenRouter::new("sk-or-test-mock", "deepseek/deepseek-v3.2-speciale"))`
  - Wire into `into_server_deps()`

- [x] **Update `.env.example`**
  - Add `OPENROUTER_API_KEY=sk-or-v1-your-key-here`

- [x] **Verify**: `cargo check` passes with new ServerDeps field

### Phase 2: Switch Tier 1 Call Sites — Crawling Domain

These 4 functions in `domains/crawling/activities/post_extraction.rs` currently create their own `OpenAi::from_env("gpt-4o")` client. Refactor to accept `ai_next` from deps.

- [x] **`extract_narrative_posts()`** — change `OpenAi::from_env("gpt-4o")?` to use `&OpenRouter` parameter; caller passes `deps.ai_next.as_ref()`; change model arg in `.extract()` to `FRONTIER_MODEL`

- [x] **`dedupe_and_merge_posts()`** — same pattern: accept `&OpenRouter`, use `FRONTIER_MODEL`

- [x] **`investigate_post()`** — two changes:
  1. The `.extract()` structured output call: use `&OpenRouter` + `FRONTIER_MODEL`
  2. The agent tool-calling section: `(*ai_next).clone().tool(WebSearchTool).tool(FetchPageTool).prompt(...).preamble(...).multi_turn(5).send()` — already works since `OpenRouter` implements `Agent`

- [x] **Update callers** of these 3 functions to pass `deps.ai_next.as_ref()` instead of constructing `OpenAi::from_env()`

### Phase 3: Switch Tier 1 Call Sites — Posts Domain

- [x] **`deduplication.rs: deduplicate_posts_llm()`** — change param `ai: &OpenAi` → `ai: &OpenRouter`; change model in `.extract()` to `FRONTIER_MODEL`; update all callers to pass `deps.ai_next.as_ref()`

- [x] **`deduplication.rs: stage_cross_source_dedup()`** and related cross-source helpers — same pattern: `&OpenRouter` + `FRONTIER_MODEL`

- [x] **`llm_sync.rs: sync_posts_with_org()`** — change param to `ai: &OpenRouter`; model to `FRONTIER_MODEL`; update callers

- [x] **`llm_sync.rs: sync_posts_for_org()`** — same pattern

- [x] **`post_extraction.rs: extract_posts_raw()` and `extract_posts_batch()`** — change to use `&OpenRouter` + `FRONTIER_MODEL`

- [x] **`core.rs: submit_post()` / classification** — N/A: only calls `generate_summary()` (Tier 2), stays on GPT-4o

- [x] **Update Restate workflow callers** in `restate/workflows/deduplicate_posts.rs` — pass `self.deps.ai_next.as_ref()` for Tier 1 functions, keep `self.deps.ai.as_ref()` for Tier 2 (`apply_dedup_results` → `generate_merge_reason`)

### Phase 4: Switch Tier 1 Call Sites — Other Domains

- [x] **`source/activities/extract_social.rs: extract_posts_from_social()`** — change `deps.ai.clone()` → `deps.ai_next.clone()`; model to `FRONTIER_MODEL`

- [x] **`source/restate/workflows/regenerate_social_posts.rs`** — pass `ai_next` to llm_sync

- [x] **`website/restate/workflows/regenerate_posts.rs`** — pass `ai_next` to llm_sync

- [x] **`organization/restate/workflows/extract_org_posts.rs`** — pass `ai_next` to llm_sync

- [x] **`crawling/activities/org_extraction.rs`** — `deps.ai.extract()` → `deps.ai_next.extract()` with `FRONTIER_MODEL`

- [x] **`common/pii/llm_detector.rs: detect_pii_with_ai()`** — change param to `&OpenRouter`; model to `FRONTIER_MODEL`; update callers + `HybridPiiDetector` now stores `Arc<OpenRouter>` instead of API key

- [x] **`agents/activities/responses.rs: generate_reply()`** — change `(*deps.ai).clone()` → `(*deps.ai_next).clone()` for the agent tool-calling section

- [x] **`notes/activities/extraction.rs: extract_notes()`** — (not in original plan) change `deps.ai.extract("gpt-4o", ...)` → `deps.ai_next.extract(FRONTIER_MODEL, ...)`

### Phase 5: Verify Tier 2 Unchanged

Confirm these 5 call sites still use `deps.ai` (GPT-4o):

- [x] `posts/activities/post_extraction.rs: generate_summary()` — uses `deps.ai.complete()` ✓ Verified unchanged
- [x] `posts/activities/post_extraction.rs: generate_outreach_copy()` — uses `deps.ai.complete()` ✓ Verified unchanged
- [x] `agents/activities/responses.rs: generate_greeting()` — uses `deps.ai.complete()` ✓ Verified unchanged
- [x] `posts/activities/deduplication.rs: generate_merge_reason()` — uses `ai.chat_completion()` ✓ Verified unchanged
- [x] `website/activities/approval.rs: generate_assessment()` — uses `deps.ai.complete()` ✓ Verified unchanged

### Phase 6: Verify & Test

- [x] `cargo check` — clean compile ✓
- [x] `cargo test -p ai-client` — all tests pass ✓
- [x] `cargo test -p server --lib` — all tests pass (23 passed, 0 failed) ✓
- [x] Verify no remaining `"gpt-4o"` in Tier 1 call sites — grep confirmed only in server.rs constructor, test mock, and embeddings binary ✓

## Technical Considerations

**No shared trait needed**: `OpenAi` and `OpenRouter` have identical convenience methods (`extract<T>`, `chat_completion`, `complete`) but they're not behind a shared trait. Since Tier 1 and Tier 2 functions are distinct, each can accept its concrete type. No trait object or generic indirection needed.

**CompletionExt not needed for OpenRouter**: The `CompletionExt` trait in `llm_request.rs` is only used by Tier 2 `complete()` calls. Tier 1 functions use `extract()` and the `Agent` trait's `prompt().send()` pattern. No changes to CompletionExt.

**Restate handles retries**: Workflow activities are automatically retried by Restate on failure. No custom retry logic for OpenRouter is needed — if a call fails, Restate will retry the entire activity.

**`OpenAIExtractionService` is unrelated**: The `deps.extraction` field uses the extraction library's own AI wrapper — it doesn't go through `deps.ai`. No changes needed there.

## Files Changed

### Must Modify

| File | Change |
|------|--------|
| `kernel/mod.rs` | Add `FRONTIER_MODEL` const + `OpenRouter` re-export |
| `kernel/deps.rs` | Add `ai_next: Arc<OpenRouter>` field |
| `bin/server.rs` | Load `OPENROUTER_API_KEY`, create OpenRouter client |
| `kernel/test_dependencies.rs` | Add `ai_next` field + mock |
| `.env.example` | Add `OPENROUTER_API_KEY` |
| `crawling/activities/post_extraction.rs` | 3 functions: accept `&OpenRouter`, use `FRONTIER_MODEL` |
| `posts/activities/deduplication.rs` | Tier 1 functions: `&OpenRouter` + `FRONTIER_MODEL` |
| `posts/activities/llm_sync.rs` | Both sync functions: `&OpenRouter` + `FRONTIER_MODEL` |
| `posts/activities/post_extraction.rs` | `extract_posts_raw/batch`: `&OpenRouter` + `FRONTIER_MODEL` |
| `posts/activities/core.rs` | `submit_post`: use `deps.ai_next` |
| `posts/restate/workflows/deduplicate_posts.rs` | Pass `ai_next` for Tier 1, `ai` for Tier 2 |
| `source/activities/extract_social.rs` | Use `deps.ai_next` + `FRONTIER_MODEL` |
| `source/restate/workflows/regenerate_social_posts.rs` | Pass `ai_next` to sync |
| `website/restate/workflows/regenerate_posts.rs` | Pass `ai_next` to sync |
| `organization/restate/workflows/extract_org_posts.rs` | Pass `ai_next` to sync |
| `crawling/activities/org_extraction.rs` | Use `deps.ai_next` + `FRONTIER_MODEL` |
| `common/pii/llm_detector.rs` | Accept `&OpenRouter` + `FRONTIER_MODEL` |
| `agents/activities/responses.rs` | `generate_reply`: use `deps.ai_next` for agent |

### No Changes Needed

| File | Reason |
|------|--------|
| `kernel/llm_request.rs` | CompletionExt only used by Tier 2 (GPT-4o) |
| `kernel/extraction_service.rs` | Uses extraction library's own wrapper |
| `posts/activities/search.rs` | Embeddings — stays on OpenAI |
| `website/activities/mod.rs` | Embeddings — stays on OpenAI |
| `bin/generate_embeddings.rs` | Embeddings — stays on OpenAI |

## References

- Brainstorm: `docs/brainstorms/2026-02-11-wire-up-openrouter-v3-speciale-brainstorm.md`
- OpenRouter guide: `docs/guides/OPENROUTER_INTEGRATION.md`
- ai-client OpenRouter module: `packages/ai-client/src/openrouter/mod.rs`
- DeepSeek V3.2-Speciale on OpenRouter: https://openrouter.ai/deepseek/deepseek-v3.2-speciale
