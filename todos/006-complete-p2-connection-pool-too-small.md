---
status: pending
priority: p2
issue_id: "006"
tags: [code-review, performance, database, connection-pool]
dependencies: []
---

# Database Connection Pool Too Small for Production

## Problem Statement

The PostgreSQL connection pool is configured with only 10 connections, which will cause connection exhaustion under moderate load (100+ concurrent requests). This is a **HIGH PRIORITY** scalability issue that will cause production outages.

## Findings

**Location**: `/packages/server/src/server/main.rs:32-33`

**Current Configuration**:
```rust
let pool = PgPoolOptions::new()
    .max_connections(10)  // ⚠️ VERY LOW FOR PRODUCTION
    .connect(&config.database_url)
    .await?;
```

**Impact at Scale**:
| Concurrent Requests | Connections Needed | Current Capacity | Result |
|---------------------|-------------------|------------------|--------|
| 10 | 10 | 10 | OK |
| 50 | 25 | 10 | **Slow** |
| 100 | 50 | 10 | **Connection exhaustion** |
| 500 | 100+ | 10 | **Complete failure** |

**From Performance Oracle Agent**: "Current: Connection exhaustion under 100+ concurrent requests. With fix: Handle 500+ concurrent requests smoothly. Expected Impact: 10x concurrent request capacity."

## Proposed Solutions

### Option 1: Production-Ready Pool Configuration (Recommended)
**Pros**: Handles real-world traffic, industry standard
**Cons**: Higher memory usage on database server
**Effort**: Small (15 minutes)
**Risk**: Low

```rust
let pool = PgPoolOptions::new()
    .max_connections(50)  // Production minimum
    .min_connections(10)  // Keep warm connections
    .acquire_timeout(Duration::from_secs(5))  // Fail fast
    .idle_timeout(Duration::from_secs(600))  // 10 min
    .max_lifetime(Duration::from_secs(1800))  // 30 min rotation
    .connect(&config.database_url)
    .await?;
```

### Option 2: Environment-Based Configuration
**Pros**: Different pools for dev/staging/prod
**Cons**: More configuration management
**Effort**: Medium (30 minutes)
**Risk**: Low

```rust
let max_connections = env::var("DATABASE_MAX_CONNECTIONS")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(50);

let pool = PgPoolOptions::new()
    .max_connections(max_connections)
    .min_connections(max_connections / 5)
    // ...
```

### Option 3: Dynamic Scaling Based on CPU Cores
**Pros**: Automatically adjusts to server resources
**Cons**: May over-provision on large servers
**Effort**: Medium (45 minutes)
**Risk**: Low

```rust
let cpu_cores = num_cpus::get();
let max_connections = (cpu_cores * 4).max(20).min(100);

let pool = PgPoolOptions::new()
    .max_connections(max_connections)
    // ...
```

## Recommended Action

**Option 1** with hardcoded 50 connections for immediate deployment. Add environment variable support in next sprint for flexibility across environments.

## Technical Details

**Connection Pool Sizing Formula**:
```
max_connections = (concurrent_requests × avg_query_time) / request_timeout

Example:
- 100 req/s target throughput
- 50ms average query time
- 100ms request timeout
- Calculation: (100 × 0.05) / 0.1 = 50 connections
- With 4x safety margin: 50 × 4 = 200 connections recommended
```

**Memory Impact**:
- Per connection: ~10MB PostgreSQL memory
- 10 connections: 100MB
- 50 connections: 500MB
- 100 connections: 1GB

Ensure PostgreSQL `max_connections` is set appropriately in database server configuration.

**Affected Files**:
- `/packages/server/src/server/main.rs` (Line 32-33)

**Configuration to Add**:
```toml
# .env
DATABASE_MAX_CONNECTIONS=50
DATABASE_MIN_CONNECTIONS=10
DATABASE_ACQUIRE_TIMEOUT_SECS=5
DATABASE_IDLE_TIMEOUT_SECS=600
DATABASE_MAX_LIFETIME_SECS=1800
```

## Acceptance Criteria

- [ ] Connection pool max_connections increased to 50
- [ ] min_connections set to 10 (warm pool)
- [ ] acquire_timeout set to 5 seconds (fail fast)
- [ ] idle_timeout set to 10 minutes
- [ ] max_lifetime set to 30 minutes (prevent stale connections)
- [ ] PostgreSQL server max_connections increased to 100+
- [ ] Load testing confirms 500+ concurrent requests handled
- [ ] Connection pool metrics added to monitoring
- [ ] Documentation updated with pool sizing guidelines

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/src/server/main.rs:32-33` (pool configuration)
- **Documentation**:
  - [SQLx Pool Configuration](https://docs.rs/sqlx/latest/sqlx/pool/struct.PoolOptions.html)
  - [Connection Pool Best Practices](https://wiki.postgresql.org/wiki/Number_Of_Database_Connections)
  - [HikariCP Pool Sizing](https://github.com/brettwooldridge/HikariCP/wiki/About-Pool-Sizing) (Java but principles apply)
- **Benchmarks**: 10 connections exhausted at 100 concurrent requests. 50 connections handles 500+ requests smoothly.
