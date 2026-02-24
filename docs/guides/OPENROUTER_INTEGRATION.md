# OpenRouter Integration Guide

## Overview

This document explores integrating OpenRouter as an alternative or replacement for direct OpenAI API usage in the Emergency Resource Aggregator platform.

## What is OpenRouter?

OpenRouter is a unified API gateway that provides access to multiple LLM providers through a single interface. Instead of integrating with each provider separately, you make calls to OpenRouter which routes requests to your chosen model.

**Supported Providers:**
- OpenAI (GPT-4, GPT-4o, GPT-3.5)
- Anthropic (Claude 3 Opus, Sonnet, Haiku)
- Google (Gemini Pro, Gemini Ultra)
- Meta (Llama models)
- Mistral AI
- Cohere
- And many more...

**Website:** https://openrouter.ai

## Why Consider OpenRouter?

### Benefits for this Project

1. **Cost Optimization**
   - Compare prices across providers in real-time
   - Route to cheaper models for non-critical tasks
   - Fall back to alternative providers if primary is unavailable
   - Current: GPT-4o at ~$5-15/1M tokens (input/output)
   - Alternatives: Claude Sonnet 3.5 (~$3/1M), Gemini (~$1.25/1M)

2. **Model Flexibility**
   - Test different models without code changes
   - A/B test extraction quality across providers
   - Use different models for different tasks (e.g., Claude for reasoning, GPT for structured output)

3. **Resilience**
   - Automatic failover if OpenAI is down
   - Avoid vendor lock-in
   - Better rate limit management across providers

4. **Simplified Integration**
   - Single API key for all providers
   - Consistent interface regardless of underlying model
   - Easy to switch models via configuration

### Specific Use Cases for Need Extraction

Our current AI usage (`packages/server/src/domains/organization/effects/ai_effects.rs:2`):
- **Task:** Extract structured volunteer needs from scraped website content
- **Model:** GPT-4o
- **Output:** JSON with title, TLDR, description, contact, urgency

**OpenRouter could enable:**
- Using Claude Sonnet 3.5 for better reasoning on ambiguous needs
- Using GPT-4o-mini for simple extractions (5x cheaper)
- Falling back to Gemini if OpenAI has an outage
- A/B testing to find the most accurate/cost-effective model

## Current Architecture

### AI Integration Point

The platform has a **single AI touchpoint**:

```rust
// packages/server/src/domains/organization/effects/ai_effects.rs

use rig::providers::openai;

impl NeedExtractor {
    pub fn new(api_key: String) -> Self {
        let client = openai::Client::new(&api_key);
        let agent = client
            .agent("gpt-4o")
            .preamble(EXTRACTION_PROMPT)
            .build();
        Self { agent }
    }
}
```

### Configuration

Environment variables (`packages/server/.env.example`):
```bash
OPENAI_API_KEY=sk-...
```

Configuration struct (`packages/server/src/config.rs`):
```rust
pub struct Config {
    pub openai_api_key: String,
    // ...
}
```

## Integration Options

### Option 1: OpenRouter via rig-core (Recommended)

**Check if rig-core supports OpenRouter:**

Rig-core is a Rust LLM framework that may already support multiple providers. Need to verify:

```bash
cd packages/server
cargo tree | grep rig
# Check rig-core documentation
```

**If supported**, minimal code changes:

```rust
// packages/server/src/domains/organization/effects/ai_effects.rs

use rig::providers::openrouter; // If available

impl NeedExtractor {
    pub fn new(api_key: String, model: String) -> Self {
        let client = openrouter::Client::new(&api_key);
        let agent = client
            .agent(&model) // "openai/gpt-4o", "anthropic/claude-3.5-sonnet"
            .preamble(EXTRACTION_PROMPT)
            .build();
        Self { agent }
    }
}
```

### Option 2: OpenRouter as OpenAI-Compatible Endpoint

OpenRouter provides an OpenAI-compatible API, so existing OpenAI clients can use it with minimal changes:

```rust
// packages/server/src/domains/organization/effects/ai_effects.rs

use rig::providers::openai;

impl NeedExtractor {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        let client = openai::Client::new(&api_key)
            .with_base_url(&base_url); // https://openrouter.ai/api/v1
        let agent = client
            .agent(&model) // "openai/gpt-4o" or "anthropic/claude-3.5-sonnet"
            .preamble(EXTRACTION_PROMPT)
            .build();
        Self { agent }
    }
}
```

**Environment variables:**
```bash
AI_PROVIDER=openrouter
OPENROUTER_API_KEY=sk-or-v1-...
AI_MODEL=openai/gpt-4o
# or
AI_MODEL=anthropic/claude-3.5-sonnet
```

### Option 3: Direct HTTP Integration

If rig-core doesn't support OpenRouter, call the API directly:

```rust
// Use reqwest to call OpenRouter API
// More control but more boilerplate
```

## Migration Path

### Phase 1: Add OpenRouter Support (Non-Breaking)

1. **Add configuration options:**

```rust
// packages/server/src/config.rs

pub struct Config {
    pub ai_provider: String, // "openai" or "openrouter"
    pub openai_api_key: Option<String>,
    pub openrouter_api_key: Option<String>,
    pub ai_model: String, // "gpt-4o" or "openai/gpt-4o"
}
```

2. **Update effect to support both:**

```rust
// packages/server/src/domains/organization/effects/ai_effects.rs

impl NeedExtractor {
    pub fn new(config: &Config) -> Result<Self> {
        let agent = match config.ai_provider.as_str() {
            "openai" => {
                let client = openai::Client::new(&config.openai_api_key.unwrap());
                client.agent(&config.ai_model).preamble(PROMPT).build()
            }
            "openrouter" => {
                let client = openai::Client::new(&config.openrouter_api_key.unwrap())
                    .with_base_url("https://openrouter.ai/api/v1");
                client.agent(&config.ai_model).preamble(PROMPT).build()
            }
            _ => return Err("Invalid AI provider"),
        };
        Ok(Self { agent })
    }
}
```

3. **Update `.env.example`:**

```bash
# AI Configuration
AI_PROVIDER=openai  # or "openrouter"
AI_MODEL=gpt-4o     # or "openai/gpt-4o" for OpenRouter

# OpenAI Direct
OPENAI_API_KEY=sk-...

# OpenRouter (access to all models)
OPENROUTER_API_KEY=sk-or-v1-...
```

### Phase 2: A/B Testing (Optional)

Test extraction quality across models:

```sql
-- Add column to track which model extracted each need
ALTER TABLE needs ADD COLUMN extracted_by_model VARCHAR(50);
```

Run experiments with different models and compare:
- Accuracy of extracted data
- Admin approval rates
- Cost per extraction

### Phase 3: Production Deployment

Based on testing results:
- Switch default to best-performing model
- Keep OpenAI as fallback
- Monitor costs and quality

## Cost Analysis

### Current Costs (OpenAI GPT-4o)

**Pricing:**
- Input: $2.50 per 1M tokens
- Output: $10.00 per 1M tokens

**Typical need extraction:**
- Input: ~2,000 tokens (scraped content)
- Output: ~500 tokens (structured JSON)
- Cost per extraction: ~$0.0055

**Monthly at scale (1,000 orgs, weekly scrapes):**
- 4,000 extractions/month
- Cost: ~$22/month

### Alternative Models via OpenRouter

**GPT-4o-mini (OpenAI):**
- Input: $0.15/1M tokens
- Output: $0.60/1M tokens
- Cost per extraction: ~$0.0006 (9x cheaper)
- Monthly: ~$2.40 (90% savings)

**Claude 3.5 Sonnet (Anthropic):**
- Input: $3.00/1M tokens
- Output: $15.00/1M tokens
- Cost per extraction: ~$0.0135 (slightly more expensive)
- Monthly: ~$54
- Benefit: Potentially better reasoning on ambiguous needs

**Gemini 1.5 Pro (Google):**
- Input: $1.25/1M tokens
- Output: $5.00/1M tokens
- Cost per extraction: ~$0.0050 (similar)
- Monthly: ~$20
- Benefit: Good balance of cost/quality

**Llama 3 70B (Meta via OpenRouter):**
- Input: $0.52/1M tokens
- Output: $0.75/1M tokens
- Cost per extraction: ~$0.0014 (75% cheaper)
- Monthly: ~$5.60
- Trade-off: Open source, may need prompt tuning

## Recommended Approach

### For Immediate Use

1. **Add OpenRouter support alongside OpenAI** (Option 2 - OpenAI-compatible endpoint)
2. **Keep default as GPT-4o** for stability
3. **Make provider/model configurable** via environment variables
4. **Test in development** with alternative models

### For Cost Optimization

1. **Test GPT-4o-mini first** (same provider, 90% cheaper)
2. **Compare extraction quality** with current GPT-4o
3. **If acceptable, switch default** and monitor approval rates
4. **Keep GPT-4o as fallback** for complex cases

### For Resilience

1. **Implement automatic failover:**
   - Primary: GPT-4o via OpenAI
   - Fallback: Claude 3.5 Sonnet via OpenRouter
   - Last resort: Gemini via OpenRouter

2. **Monitor and alert:**
   - Track which model is used for each extraction
   - Alert if failover is triggered
   - Dashboard showing cost/quality by model

## Security Considerations

1. **API Key Management:**
   - Store OpenRouter key in environment variables
   - Never commit keys to version control
   - Rotate keys regularly

2. **Privacy:**
   - OpenRouter logs requests for debugging (can be disabled)
   - Review OpenRouter's privacy policy
   - Ensure scraped content doesn't contain PII (already handled)

3. **Rate Limiting:**
   - OpenRouter has its own rate limits separate from providers
   - Implement retry logic with exponential backoff
   - Monitor usage to avoid unexpected overages

## Next Steps

1. **Research rig-core documentation:**
   - Check if OpenRouter provider exists
   - Review multi-provider support

2. **Create OpenRouter account:**
   - Sign up at https://openrouter.ai
   - Get API key (free tier available)
   - Test with curl to verify connectivity

3. **Implement in development branch:**
   - Add configuration options
   - Update NeedExtractor
   - Test with sample scraping job

4. **Run quality comparison:**
   - Extract 50 needs with GPT-4o (baseline)
   - Extract same 50 needs with GPT-4o-mini
   - Extract same 50 needs with Claude 3.5 Sonnet
   - Compare admin approval rates and accuracy

5. **Document findings:**
   - Update this doc with test results
   - Recommend default model based on data
   - Create migration plan if switching

## References

- OpenRouter Documentation: https://openrouter.ai/docs
- OpenRouter Pricing: https://openrouter.ai/models
- Rig-core GitHub: https://github.com/0xPlaygrounds/rig
- Current AI Implementation: `packages/server/src/domains/organization/effects/ai_effects.rs:2`
- Configuration: `packages/server/src/config.rs`

## Related Documents

- `SPIKE_1_COMPLETE.md` - Need extraction pipeline (where AI is used)
- `PROBLEM_SOLUTION.md` - Overall platform architecture
- `plans/2026-01-27-mvp-execution-plan.md` - Roadmap context

---

**Status:** Exploration document
**Last Updated:** 2026-01-27
**Author:** Documentation
**Next Action:** Test rig-core compatibility with OpenRouter
