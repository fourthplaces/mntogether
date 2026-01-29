# intelligent-crawler

A domain-agnostic web crawler with AI-powered page evaluation and structured data extraction, designed for Seesaw event-sourced architectures.

## Architecture

This crate follows the **Seesaw** architecture pattern:

```
Seesaw State Machines (Policy - in main app)
    ↓ Commands (what to do)
Effect Handlers (Infrastructure - this crate)
    ↓ Execute IO
    ↑ Events (what happened)
Seesaw State Machines
    ↓ Make decisions
Commands...
```

### What This Crate Provides

**Infrastructure only** - no business logic or policy decisions:

- ✅ **Events** (`CrawlerEvent`) - Facts about what happened
- ✅ **Commands** (`CrawlerCommand`) - Instructions to execute
- ✅ **Effect Handlers** - Execute commands, emit events
  - `DiscoveryEffect` - Crawl websites
  - `FlaggingEffect` - Evaluate pages with AI
  - `ExtractionEffect` - Extract structured data
  - `RefreshEffect` - Monitor pages for changes
- ✅ **Traits** - Abstraction over infrastructure
  - `CrawlerStorage` - Persistence
  - `PageFetcher` - HTTP client
  - `PageEvaluator` - AI evaluation
  - `RateLimiter` - Rate limiting

### What This Crate Does NOT Provide

**Policy decisions** (belong in your main app):

- ❌ WHEN to crawl
- ❌ WHETHER to extract (confidence thresholds)
- ❌ HOW OFTEN to refresh
- ❌ WHAT the extracted data means

## Status: 🚧 Work in Progress

### ✅ Completed

- [x] Event definitions (`events.rs`)
- [x] Command definitions (`commands.rs`)
- [x] Core types (`new_types.rs`)
- [x] Trait definitions (`traits.rs`)
- [x] Effect handlers (discovery, flagging, extraction, refresh)

### 🔨 TODO

- [ ] Fix type constraints (associated types → Uuid)
- [ ] Implement PostgresStorage
- [ ] Implement HttpFetcher
- [ ] Implement RateLimiter
- [ ] Write tests
- [ ] Add example evaluator implementations

### 🏗️ In Main App (mndigitalaid/server)

- [ ] State machines (ResourceDiscoveryMachine, PageLifecycleMachine)
- [ ] Coordinator (event/command routing)
- [ ] OpportunityEvaluator (YOUR prompts, YOUR domain)
- [ ] OpportunityCrawlerAdapter (RawExtraction → Opportunity)

## Current Issue

The effect handlers have type mismatches between:
- **Commands/Events**: Use concrete `Uuid` types (for serialization)
- **Storage traits**: Use associated types (for flexibility)

**Solution**: For MVP, constrain associated types to `Uuid`. Future: add conversion traits.

## Usage (Once Complete)

```rust
use intelligent_crawler::{
    CrawlerEvent, CrawlerCommand,
    DiscoveryEffect, FlaggingEffect,
    PostgresStorage, HttpFetcher, YourEvaluator, RateLimiter,
};

// Set up infrastructure
let storage = PostgresStorage::new(pool);
let fetcher = HttpFetcher::new();
let evaluator = YourEvaluator::new(); // YOUR prompts here
let rate_limiter = RateLimiter::new();

// Create effect handlers
let discovery = DiscoveryEffect::new(storage.clone(), fetcher, evaluator.clone(), rate_limiter);
let flagging = FlaggingEffect::new(storage.clone(), evaluator);

// Execute command (from your state machine)
let cmd = CrawlerCommand::DiscoverResource {
    resource_id: uuid!("..."),
    max_depth: 2,
    same_domain_only: true,
};

// Effect handler executes and returns events
let events = discovery.execute(cmd).await?;

// Your state machine processes events
for event in events {
    let commands = your_machine.apply(event);
    // Execute commands...
}
```

## Design Principles

1. **Infrastructure, not policy** - This crate describes HOW to do things, not WHEN/WHETHER
2. **Domain-agnostic** - No knowledge of "opportunities", "volunteers", etc.
3. **Event-sourced** - All state changes captured as events
4. **Trait-based** - Easy to mock, test, swap implementations
5. **Type-safe** - Strong typing prevents mixing resource/page/extraction IDs

## Next Steps

1. Fix type constraints in effect handlers
2. Implement PostgresStorage trait
3. Create example evaluator (OpenAI/Anthropic)
4. Write integration tests
5. Document for main app integration

## License

MIT
