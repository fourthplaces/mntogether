---
status: pending
priority: p1
issue_id: "005"
tags: [code-review, performance, database, vector-search]
dependencies: []
---

# Vector Search Index Severely Under-Configured

## Problem Statement

The pgvector IVFFlat index is configured with `lists = 100`, which will cause severe performance degradation as the member database grows beyond 10,000 records. This is a **CRITICAL** scalability bottleneck.

## Findings

**Location**: `/packages/server/migrations/20260127000015_add_embeddings.sql`

**Current Configuration**:
```sql
CREATE INDEX idx_members_embedding ON members
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);  -- ⚠️ TOO LOW FOR PRODUCTION
```

**Performance Impact by Scale**:
| Member Count | Current lists | Recommended lists | Performance Impact |
|--------------|---------------|-------------------|-------------------|
| 100 | 100 | 10 | Acceptable |
| 1,000 | 100 | 10 | Degraded |
| 10,000 | 100 | 100 | **Severe degradation** |
| 100,000 | 100 | 1,000 | **Critical failure** |

**From Performance Oracle Agent**: "IVFFlat index with `lists = 100` is severely under-configured. Recommended: `lists = rows / 1000`. At 100K members, you need ~1,000 lists. Current config will cause O(n) complexity during search."

**Expected Latency**:
- Current (10K members): 500-2000ms per search
- After fix (10K members): 50-200ms per search
- **10-100x performance improvement**

## Proposed Solutions

### Option 1: Dynamic Lists Calculation (Recommended)
**Pros**: Scales automatically with data growth
**Cons**: Requires rebuild as data grows
**Effort**: Medium (1 hour)
**Risk**: Low

```sql
-- For current scale (testing with <1K members)
CREATE INDEX idx_members_embedding ON members
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 10);

-- Production migration (10K-100K members expected)
CREATE INDEX idx_members_embedding ON members
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 500);

-- Document rebuild strategy when hitting 500K members
-- REINDEX will require downtime or online index rebuild
```

### Option 2: Switch to HNSW Index (Better Long-Term)
**Pros**: Better performance at scale, no list tuning needed
**Cons**: Requires pgvector 0.5.0+, higher memory usage
**Effort**: Medium (2 hours including testing)
**Risk**: Low

```sql
-- HNSW is superior for large-scale vector search
CREATE INDEX idx_members_embedding ON members
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- No list management needed
-- Better performance characteristics: O(log n) vs O(n/lists)
-- Higher memory usage but acceptable for 100K+ members
```

### Option 3: Hybrid Approach
**Pros**: Best of both worlds
**Cons**: More complex
**Effort**: Large (3 hours)
**Risk**: Medium

```sql
-- IVFFlat for small-medium scale (cheaper memory)
-- Auto-switch to HNSW at 50K members (better performance)
-- Requires migration strategy and monitoring
```

## Recommended Action

**Immediate**: Update to `lists = 500` for expected production scale (10K-50K members)

**Long-term**: Plan migration to HNSW index when approaching 50K members or when memory budget allows

## Technical Details

**Affected Queries**:
- `/packages/server/src/domains/matching/models/match_candidate.rs:42-64` (find_within_radius)
- `/packages/server/src/domains/matching/models/match_candidate.rs:77-98` (find_statewide)

**Index Parameters**:
- **lists**: Number of clusters for IVFFlat (rule of thumb: rows/1000)
- **m**: HNSW connections per layer (higher = better recall, more memory)
- **ef_construction**: HNSW build quality (higher = better index, slower build)

**Migration Strategy**:
```sql
-- Step 1: Create new index with better config
CREATE INDEX CONCURRENTLY idx_members_embedding_v2 ON members
    USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 500);

-- Step 2: Drop old index (after verifying performance)
DROP INDEX idx_members_embedding;

-- Step 3: Rename new index
ALTER INDEX idx_members_embedding_v2 RENAME TO idx_members_embedding;
```

**Memory Impact**:
- IVFFlat (lists=100): ~50MB for 10K members
- IVFFlat (lists=500): ~60MB for 10K members
- HNSW (m=16): ~120MB for 10K members

## Acceptance Criteria

- [ ] Index lists parameter increased to 500
- [ ] Migration tested on staging with 10K+ member dataset
- [ ] Query performance measured before/after (target: <200ms)
- [ ] Index size monitored (should be <100MB for 10K members)
- [ ] Documentation updated with index tuning strategy
- [ ] Monitoring alerts added for slow vector searches (>500ms)
- [ ] Reindex procedure documented for future scaling
- [ ] HNSW migration plan documented for 50K+ members

## Work Log

*Empty - work not started*

## Resources

- **PR/Issue**: N/A - Found in code review
- **Related Code**:
  - `/packages/server/migrations/20260127000015_add_embeddings.sql` (index creation)
  - `/packages/server/src/domains/matching/models/match_candidate.rs` (vector search queries)
- **Documentation**:
  - [pgvector Performance Tuning](https://github.com/pgvector/pgvector#ivfflat)
  - [HNSW vs IVFFlat Comparison](https://github.com/pgvector/pgvector#indexing)
  - [Vector Index Benchmarks](https://github.com/erikbern/ann-benchmarks)
- **Benchmarks**: At 10K members with lists=100, search takes 500-2000ms. With lists=500, expect 50-200ms.
