---
status: pending
priority: p1
issue_id: "002"
tags: [code-review, security, rate-limiting, dos]
dependencies: []
---

# Missing Rate Limiting on Critical Endpoints

## Problem Statement

No rate limiting is implemented on authentication endpoints, GraphQL API, or user-facing mutations. This exposes the application to brute force attacks, SMS spam, API abuse, and denial-of-service attacks.

## Findings

**Security Impact**:
- **Brute force attacks on OTP codes**: 6-digit codes = 1 million combinations, no throttling
- **SMS spam/DoS**: Unlimited OTP requests can drain Twilio credits
- **API abuse**: No protection against resource exhaustion
- **Cost escalation**: Uncapped external API usage (Twilio, OpenAI)

**Affected Endpoints**:
1. `/auth/send-code` - OTP code sending (Twilio SMS)
2. `/auth/verify-code` - OTP verification
3. `/graphql` - All mutations (submitNeed, approveNeed, etc.)
4. GraphQL queries (potential resource exhaustion)

**From Security Sentinel Agent**: "No rate limiting implementation found for OTP code sending, OTP verification, GraphQL mutations, or user registration."

## Proposed Solutions

### Option 1: Tower Governor Middleware (Recommended)
**Pros**: Battle-tested, minimal code, per-IP limiting
**Cons**: In-memory only (not distributed)
**Effort**: Medium (2 hours)
**Risk**: Low

```rust
use tower_governor::{GovernorLayer, GovernorConfigBuilder};

// OTP endpoints: 5 attempts per 5 minutes per IP
let otp_limiter = GovernorConfigBuilder::default()
    .per_second(1)
    .burst_size(5)
    .finish()
    .unwrap();

// GraphQL: 100 requests per minute per IP
let graphql_limiter = GovernorConfigBuilder::default()
    .per_second(10)
    .burst_size(20)
    .finish()
    .unwrap();

let app = Router::new()
    .route("/auth/send-code", post(send_code)
        .layer(GovernorLayer::new(Arc::new(otp_limiter))))
    .route("/graphql", post(graphql_handler)
        .layer(GovernorLayer::new(Arc::new(graphql_limiter))));
```

### Option 2: Redis-Backed Rate Limiting
**Pros**: Distributed, works across multiple servers
**Cons**: Requires Redis dependency
**Effort**: Large (4 hours)
**Risk**: Medium

```rust
use redis::AsyncCommands;

async fn check_rate_limit(redis: &Client, key: &str, limit: u32, window: u64) -> Result<bool> {
    let mut conn = redis.get_async_connection().await?;
    let count: u32 = conn.incr(&key, 1).await?;
    if count == 1 {
        conn.expire(&key, window as usize).await?;
    }
    Ok(count <= limit)
}
```

### Option 3: Database-Backed Rate Limiting
**Pros**: No new dependencies, persistent
**Cons**: Database load, slower than in-memory
**Effort**: Large (6 hours)
**Risk**: Medium-High

## Recommended Action

**Option 1** (Tower Governor) for immediate deployment. This provides solid protection with minimal complexity. Consider Option 2 (Redis) when scaling to multiple servers.

## Technical Details

**Rate Limit Strategy**:
- **OTP Send**: 5 attempts per 5 minutes per IP
- **OTP Verify**: 10 attempts per 5 minutes per IP
- **GraphQL Mutations**: 100 per minute per authenticated user
- **GraphQL Queries**: 200 per minute per IP

**Affected Files**:
- `/packages/server/src/server/app.rs` (middleware)
- `/packages/server/src/server/auth/edges.rs` (OTP endpoints)

**Error Responses**:
```json
{
  "error": "Rate limit exceeded. Try again in 2 minutes.",
  "retry_after": 120
}
```

## Acceptance Criteria

- [ ] Rate limiting implemented on OTP send endpoint (5/5min)
- [ ] Rate limiting implemented on OTP verify endpoint (10/5min)
- [ ] Rate limiting implemented on GraphQL endpoint (100/min)
- [ ] Proper HTTP 429 responses with Retry-After header
- [ ] Rate limits configurable via environment variables
- [ ] Different limits for authenticated vs. anonymous users
- [ ] Monitoring/logging of rate limit hits
- [ ] Documentation of rate limits in API guide

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/src/server/auth/edges.rs:9` (OTP sending)
  - `/packages/server/src/server/app.rs` (middleware setup)
- **Documentation**:
  - [Tower Governor](https://docs.rs/tower-governor/)
  - [OWASP Rate Limiting](https://owasp.org/www-community/controls/Blocking_Brute_Force_Attacks)
- **Cost Analysis**: Uncapped SMS sending could cost $100s-$1000s in Twilio charges during an attack
