# PII Scrubbing Architecture

## Overview

Regex-based PII detection and redaction. Catches structured PII (emails, phones, SSNs, credit cards, IPs) with context-aware filtering that preserves public organizational contact info.

Currently used in post extraction — scrubs scraped content before AI processing and storage.

## Design

1. **Models Stay Pure**: No PII logic in models
2. **Effect-Layer Scrubbing**: Detection/redaction happens before calling models
3. **Trait-Based**: `BasePiiDetector` trait in `ServerDeps` for dependency injection
4. **Context-Aware**: `PersonalMessage` scrubs aggressively; `PublicContent` preserves org contact info

## Detection Context

```rust
pub enum DetectionContext {
    PersonalMessage,  // Scrub ALL PII
    PublicContent,    // Preserve public org contact info (info@, contact@, etc.)
}
```

`PublicContent` checks surrounding text for organizational keywords ("contact us", "office", "headquarters") and skips emails/phones that appear in that context.

## Redaction Strategies

```rust
pub enum RedactionStrategy {
    FullRemoval,       // "john@example.com" → "[REDACTED]"
    PartialMask,       // "john@example.com" → "j***@example.com"
    TokenReplacement,  // "john@example.com" → "[EMAIL]"
}
```

## Implementations

| Detector | Use |
|----------|-----|
| `RegexPiiDetector` | Production — regex detection, <1ms |
| `NoopPiiDetector` | When scrubbing is disabled, or in tests |

Factory: `create_pii_detector(enabled: bool) -> Arc<dyn BasePiiDetector>`

## Configuration

```bash
PII_SCRUBBING_ENABLED=true  # default: true
```

## File Structure

```
packages/server/src/
├── common/pii/
│   ├── mod.rs          # Public API exports
│   ├── detector.rs     # Regex detection + context filtering
│   └── redactor.rs     # Redaction strategies
└── kernel/
    ├── traits.rs       # BasePiiDetector trait
    └── pii.rs          # RegexPiiDetector, NoopPiiDetector, factory
```

## Detection Coverage

| PII Type | Accuracy |
|----------|----------|
| Email addresses | 98%+ |
| Phone numbers | 95%+ |
| SSNs | 99%+ |
| Credit cards (Luhn-validated) | 99%+ |
| IP addresses | 99%+ |
