---
date: 2026-02-11
topic: wire-up-openrouter-v3-speciale
---

# Wire Up OpenRouter with DeepSeek V3.2-Speciale

## What We're Building

Add a second AI client (`ai_next`) to ServerDeps backed by OpenRouter's DeepSeek V3.2-Speciale model. Switch 19 accuracy-critical call sites from GPT-4o to V3.2-Speciale for higher accuracy at dramatically lower cost. Keep GPT-4o for 5 hallucination-tolerant tasks and all embeddings.

## Why This Approach

**Model choice: DeepSeek V3.2-Speciale** (`deepseek/deepseek-v3.2-speciale`)
- "Reported evaluations place Speciale ahead of GPT-5 on difficult reasoning workloads"
- Optimized for reasoning and agentic tool use via reinforcement learning
- $0.27/$0.41 per 1M tokens vs GPT-4o at $2.50/$10.00 (9x/24x cheaper)
- Full structured output (json_schema) and tool calling support on OpenRouter
- 163K context window

**Rejected alternatives:**
- **Kimi K2.5** — Output tokens 5.5x more expensive than DS; documented tool calling issues on OpenRouter
- **DeepSeek V3.2 base** — Only 8% cheaper than Speciale but meaningfully less capable on reasoning
- **Single model approach** — GPT-4o isn't good enough for critical tasks; V3.2-Speciale is the upgrade

## Key Decisions

- **Required, not optional**: OPENROUTER_API_KEY must be set. No fallback, no Option<> handling.
- **Named `ai_next`**: `deps.ai` stays as GPT-4o/OpenAI (embeddings + simple tasks), `deps.ai_next` is V3.2-Speciale.
- **Drop-in swap**: OpenRouter module has identical convenience methods (`extract<T>`, `chat_completion`, `complete`, tool calling). Each call site is a simple `deps.ai` → `deps.ai_next` change.

## Call Site Tiers

### Tier 1: Switch to `deps.ai_next` (V3.2-Speciale) — 19 call sites

| Domain | File | Function | Type |
|--------|------|----------|------|
| Crawling | `post_extraction.rs` | `extract_narrative_posts` | Structured output |
| Crawling | `post_extraction.rs` | `dedupe_and_merge_posts` | Structured output |
| Crawling | `post_extraction.rs` | `investigate_post` (extraction) | Structured output |
| Crawling | `post_extraction.rs` | `investigate_post` (agent) | Agent + tools |
| Posts | `post_extraction.rs` | `extract_posts_raw` | Structured output |
| Posts | `post_extraction.rs` | `extract_posts_batch` | Structured output |
| Posts | `deduplication.rs` | `deduplicate_posts_llm` | Structured output |
| Posts | `deduplication.rs` | `stage_cross_source_dedup` | Structured output |
| Posts | `llm_sync.rs` | `sync_posts_with_org` | Structured output |
| Posts | `llm_sync.rs` | `sync_posts_for_org` | Structured output |
| Posts | `core.rs` | `submit_post` (classification) | Structured output |
| Posts | `deduplicate_posts.rs` | `find_duplicate_pending` | Structured output |
| Posts | `deduplicate_posts.rs` | `match_pending_to_active` | Structured output |
| Source | `extract_social.rs` | `extract_posts_from_social` | Structured output |
| Source | `regenerate_social_posts.rs` | `llm_sync_posts` | Structured output |
| Website | `regenerate_posts.rs` | `llm_sync_posts` | Structured output |
| Organization | `extract_org_posts.rs` | `llm_sync_posts_for_org` | Structured output |
| PII | `llm_detector.rs` | `detect_pii_with_ai` | Structured output |
| Agents | `responses.rs` | `generate_reply` | Agent + tools |

### Tier 2: Keep on `deps.ai` (GPT-4o) — 5 call sites

| File | Function | Why GPT-4o |
|------|----------|-----------|
| `post_extraction.rs` | `generate_summary` | Just a 2-3 sentence summary |
| `post_extraction.rs` | `generate_outreach_copy` | Email template, human edits |
| `responses.rs` | `generate_greeting` | Simple welcome message |
| `deduplication.rs` | `generate_merge_reason` | Explanation text for admins |
| `approval.rs` | `generate_assessment` | Markdown summary of research |

### Must stay OpenAI direct — 5 call sites

All `create_embedding` / `embed` calls use `text-embedding-3-small` — no OpenRouter equivalent.

## Implementation Shape

```rust
// ServerDeps
pub ai: Arc<OpenAi>,          // GPT-4o + embeddings
pub ai_next: Arc<OpenRouter>,  // V3.2-Speciale

// Config
pub openrouter_api_key: String,  // Required

// Construction (bin/server.rs)
let ai_next = Arc::new(
    OpenRouter::new(config.openrouter_api_key, "deepseek/deepseek-v3.2-speciale")
        .with_app_name("MN Together")
        .with_site_url("https://mntogether.org")
);
```

## Open Questions

- How does V3.2-Speciale handle the exact structured output schemas we use? (test with real extraction)
- Latency comparison: OpenRouter adds a hop — is latency acceptable for the investigation agent?
- Rate limits on OpenRouter for the V3.2-Speciale endpoint?

## Next Steps

-> `/workflows:plan` for implementation details
