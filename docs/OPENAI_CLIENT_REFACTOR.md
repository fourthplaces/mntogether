# AI Client Refactoring (Historical)

> **Note**: This document describes a completed refactoring. The original `openai-client` package has been superseded by the `ai-client` package, which provides a unified LLM abstraction supporting both OpenAI and OpenRouter (Claude, GPT).

## Summary

The AI client code was extracted into a reusable package to separate API concerns from domain logic.

## Current Package: `ai-client`

The `ai-client` package (`packages/ai-client/`) provides:

- LLM client abstraction for structured AI calls
- Support for OpenAI and OpenRouter providers
- Used by the server for AI-powered features (PII detection, summary generation, editorial notes)

## Architecture

```
server (main crate)
└── ai-client (LLM abstraction)
    ├── OpenAI provider
    └── OpenRouter provider (Claude, GPT, etc.)
```

## Benefits

- **Clean separation** — AI client has no domain-specific logic
- **Provider agnostic** — Switch between OpenAI and OpenRouter
- **Testable** — Pure client is easy to mock
- **Maintainable** — AI provider changes don't affect domain code
