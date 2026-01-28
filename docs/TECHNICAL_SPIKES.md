# Technical Spikes - De-Risking the Architecture

## Overview

Spikes are time-boxed experiments to reduce technical uncertainty before committing to full implementation. Each spike has:
- **Goal**: What we're trying to prove/learn
- **Success Criteria**: How we know it works
- **Time Box**: Maximum time to spend
- **Deliverable**: Code artifact or written findings

---

## Priority 1: High Risk, High Impact

### SPIKE 1: GraphQL Subscriptions with Juniper + Redis

**Risk**: Juniper's subscription support may be immature, or wiring Redis pub/sub to GraphQL subscriptions might be complex.

**Goal**: Prove we can implement real-time GraphQL subscriptions using Juniper with Redis pub/sub as the backend.

**Time Box**: 2-3 hours

**Tasks**:
1. Set up minimal Juniper subscription (hello world)
2. Connect Redis pub/sub to subscription stream
3. Test with multiple clients (browser WebSocket connections)
4. Verify messages broadcast to all subscribers

**Success Criteria**:
- [ ] GraphQL subscription endpoint works (`ws://localhost:8080/graphql`)
- [ ] Redis PUBLISH triggers GraphQL subscription events
- [ ] Multiple clients receive same event simultaneously
- [ ] Clean shutdown (no hanging connections)

**Deliverable**:
```rust
// spike_graphql_subscriptions/
// - Basic Juniper subscription setup
// - Redis pub/sub integration
// - Test client (HTML + Apollo Client)
```

**Acceptance**: If this works cleanly, proceed with chat architecture. If Juniper subscriptions are buggy, consider switching to Server-Sent Events (SSE) instead.

---

### SPIKE 2: rig.rs Conversational AI (Intake Flow)

**Risk**: Building a good intake conversation requires careful prompt engineering. The AI might ask bad questions, loop, or fail to extract useful information.

**Goal**: Prove we can build a natural, effective intake conversation that extracts quality `searchable_text` from volunteers.

**Time Box**: 3-4 hours

**Tasks**:
1. Design intake prompt with conversation rules
2. Build simple REPL that simulates intake conversation
3. Test with 5 different volunteer personas:
   - Bilingual lawyer with specific availability
   - College student with vague interests
   - Retiree with medical background
   - Single parent with limited time
   - Spanish-speaking volunteer
4. Evaluate quality of extracted profile text

**Success Criteria**:
- [ ] AI asks 3-5 relevant questions per conversation
- [ ] Conversation feels natural (not robotic)
- [ ] Extracted text includes: skills, availability, location, interests
- [ ] AI gracefully handles vague answers ("I'm not sure...")
- [ ] Conversation completes in 5-10 exchanges

**Deliverable**:
```rust
// spike_intake_ai/
// - intake_prompt.txt (tuned prompt)
// - intake_repl.rs (CLI conversation simulator)
// - test_conversations/ (5 example conversations with personas)
// - evaluation.md (quality assessment)
```

**Evaluation Template**:
```markdown
## Conversation with Persona: Bilingual Lawyer

**Extracted Profile**:
"Bilingual lawyer (English/Spanish) specializing in immigration law. Available weekends
and Wednesday evenings. Based in Minneapolis. Interested in pro bono legal aid and
document translation for immigrant communities."

**Quality Score**: 4/5
- ✅ Clear skills (legal, bilingual)
- ✅ Specific availability (weekends, Wed evenings)
- ✅ Location (Minneapolis)
- ✅ Interests (pro bono, translation)
- ❌ Could probe for specific legal areas

**AI Questions Asked**:
1. "What kind of volunteering interests you most?"
2. "Do you have any specific skills you'd like to use?"
3. "When are you typically available to volunteer?"
4. "Are you located in the Twin Cities area?"
5. "Any particular communities or causes you're passionate about?"
```

**Acceptance**: If 4+ conversations score 3+/5, proceed with intake AI. If quality is poor, redesign prompt or consider simpler form-based intake.

---

### SPIKE 3: Content Hash Synchronization (Duplicate Detection)

**Risk**: SHA256 hashing of normalized text might produce too many false positives (minor edits flagged as new needs) or false negatives (same need worded differently).

**Goal**: Prove content hash approach reliably detects duplicates AND changes without excessive false positives/negatives.

**Time Box**: 2 hours

**Tasks**:
1. Implement `generate_content_hash()` with normalization
2. Create test dataset:
   - 10 identical needs with minor variations (punctuation, case, spacing)
   - 5 semantically identical needs worded differently
   - 5 similar but distinct needs
3. Test hash collisions and mismatches
4. Tune normalization rules if needed

**Test Cases**:
```rust
// Should match (same hash):
"We need Spanish-speaking volunteers!"
"We need spanish speaking volunteers"
"we need spanish speaking volunteers!!!"

// Should NOT match (different hashes):
"We need Spanish-speaking volunteers"
"We need French-speaking volunteers"

// Edge case (should these match?):
"Volunteers needed for food distribution"
"Food distribution volunteers needed"
```

**Success Criteria**:
- [ ] 10 identical variations produce same hash (0% false negative)
- [ ] 5 distinct needs produce different hashes (0% false positive)
- [ ] Decision on word-order normalization (sort words or not?)
- [ ] Hash generation takes < 1ms per text

**Deliverable**:
```rust
// spike_content_hash/
// - content_hash.rs (implementation)
// - test_dataset.json (20+ test cases)
// - evaluation.md (findings + recommendations)
```

**Acceptance**: If false positive rate < 5% and false negative rate < 2%, proceed with hash-based sync. If rates are high, consider semantic embeddings for duplicate detection instead.

---

## Priority 2: Medium Risk, High Impact

### SPIKE 4: Expo Push Notifications from Rust

**Risk**: Expo push notification API integration from Rust might have gotchas. Delivery reliability unknown.

**Goal**: Prove we can send Expo push notifications from Rust backend reliably.

**Time Box**: 1-2 hours

**Tasks**:
1. Register test Expo app, get push token
2. Implement Expo push API client in Rust (HTTP POST)
3. Send test notifications
4. Verify delivery to physical device
5. Test error handling (invalid token, rate limits)

**Success Criteria**:
- [ ] Push notification received on physical device
- [ ] Notification displays title + body correctly
- [ ] Deep linking works (tap notification → open app)
- [ ] Error handling for invalid tokens
- [ ] Batch sending (100+ notifications) works

**Deliverable**:
```rust
// spike_expo_push/
// - expo_client.rs (Expo API client)
// - test_push.rs (send test notification)
// - findings.md (latency, error cases, rate limits)
```

**Acceptance**: If notifications deliver reliably (>95% success rate), proceed. If delivery is flaky, investigate alternatives (FCM/APNs directly).

---

### SPIKE 5: Firecrawl + AI Need Extraction

**Risk**: Websites have messy HTML. AI extraction might hallucinate needs or miss real ones.

**Goal**: Prove we can reliably extract real volunteer needs from organization websites.

**Time Box**: 2-3 hours

**Tasks**:
1. Pick 5 diverse organization websites (churches, nonprofits, community centers)
2. Scrape with Firecrawl
3. Extract needs with rig.rs + GPT-4o
4. Manually verify results (precision/recall)

**Test Websites**:
```
1. https://www.ascensionburnsville.org/ (Lutheran church)
2. https://arriveministries.org/ (Refugee services)
3. https://www.templeisrael.com/ (Reform synagogue)
4. https://holycross-pl.org/ (ESL classes)
5. https://mnicom.org/ (Immigration coalition)
```

**Success Criteria**:
- [ ] Firecrawl successfully scrapes all 5 websites
- [ ] AI extracts 3-8 needs per site (not 0, not 50)
- [ ] Precision: >80% (extracted needs are real)
- [ ] Recall: >60% (doesn't miss obvious needs)
- [ ] No hallucinations (made-up needs)

**Evaluation Template**:
```markdown
## Website: Arrive Ministries

**Extracted Needs**:
1. "English tutors for refugee families (ongoing)" ✅ REAL
2. "Drivers to transport clients to appointments" ✅ REAL
3. "Volunteer coordinators needed" ✅ REAL
4. "Donation sorters for clothing closet" ✅ REAL

**Missed Needs**:
- "Legal advocacy volunteers" (mentioned in footer) ❌ MISSED

**Precision**: 4/4 = 100%
**Recall**: 4/5 = 80%
```

**Deliverable**:
```rust
// spike_need_extraction/
// - firecrawl_client.rs (scraper)
// - extraction_prompt.txt (GPT-4o prompt)
// - test_results/ (5 website evaluations)
// - evaluation.md (overall precision/recall)
```

**Acceptance**: If precision >70% and recall >50%, proceed. If quality is poor, tune prompts or add human-in-the-loop verification step.

---

### SPIKE 6: pgvector HNSW Performance

**Risk**: Vector search might be too slow at scale (10K+ volunteers, 1K+ needs).

**Goal**: Prove pgvector with HNSW indexes is fast enough for production.

**Time Box**: 1-2 hours

**Tasks**:
1. Generate synthetic dataset (10K volunteer embeddings, 1K need embeddings)
2. Create HNSW index on volunteers table
3. Benchmark top-20 similarity search (1000 queries)
4. Test with different HNSW parameters (m, ef_construction)

**Success Criteria**:
- [ ] Top-20 search completes in < 50ms (p95)
- [ ] Throughput: >100 queries/sec
- [ ] Index build time: < 5 minutes for 10K vectors
- [ ] Memory usage: < 500MB for 10K vectors

**Deliverable**:
```rust
// spike_pgvector_perf/
// - generate_dataset.rs (synthetic embeddings)
// - benchmark.rs (query performance test)
// - findings.md (latency, throughput, tuning params)
```

**Acceptance**: If p95 latency < 100ms, proceed with pgvector. If slow, investigate alternatives (Qdrant, Weaviate).

---

## Priority 3: Low Risk, Nice to Validate

### SPIKE 7: Redis Broadcast Across Multiple Servers

**Risk**: Redis pub/sub might not work cleanly with multiple Rust server instances (message duplication, dropped messages).

**Goal**: Prove Redis pub/sub reliably broadcasts messages across multiple server instances.

**Time Box**: 1 hour

**Tasks**:
1. Start 3 Rust server instances on different ports
2. Subscribe all 3 to same Redis channel
3. Publish message from one server
4. Verify all 3 receive message exactly once
5. Test with 100+ messages/sec load

**Success Criteria**:
- [ ] All subscribers receive every message
- [ ] No duplicate messages
- [ ] No dropped messages under load
- [ ] Graceful handling of subscriber crashes

**Deliverable**:
```rust
// spike_redis_broadcast/
// - multi_server_test.rs (spawn 3 servers, test broadcast)
// - findings.md (reliability, edge cases)
```

**Acceptance**: If 100% message delivery with no duplicates, proceed. If unreliable, investigate NATS or alternative message bus.

---

### SPIKE 8: GraphQL Mutation Performance (Batching)

**Risk**: Sending 100+ push notifications in a single GraphQL mutation might timeout or OOM.

**Goal**: Prove we can handle batch operations (send 100+ notifications) without performance issues.

**Time Box**: 1 hour

**Tasks**:
1. Create mutation that sends N notifications
2. Test with N = 1, 10, 100, 500
3. Measure latency, memory usage
4. Identify bottlenecks (database, Expo API, etc.)

**Success Criteria**:
- [ ] 100 notifications complete in < 5 seconds
- [ ] Memory usage stays < 100MB per request
- [ ] No database connection pool exhaustion
- [ ] Expo API rate limits respected

**Deliverable**:
```rust
// spike_batch_mutations/
// - batch_notify.rs (send N notifications)
// - benchmark_results.md (latency at different scales)
```

**Acceptance**: If 100 notifications complete in <10s, proceed. If slow, implement job queue (e.g., Faktory) for background processing.

---

## Spike Execution Plan

### Week 1: Critical Path Spikes

**Day 1-2**:
- ✅ SPIKE 1: GraphQL Subscriptions (3h)
- ✅ SPIKE 2: Intake AI (4h)

**Day 3**:
- ✅ SPIKE 3: Content Hash (2h)
- ✅ SPIKE 4: Expo Push (2h)

**Day 4**:
- ✅ SPIKE 5: Need Extraction (3h)
- ✅ SPIKE 6: pgvector Performance (2h)

**Day 5**:
- ✅ SPIKE 7: Redis Broadcast (1h)
- ✅ SPIKE 8: Batch Mutations (1h)

**Total**: ~18 hours over 5 days

### Decision Points

After each spike, decide:
1. **Green** (✅) - Proceed as designed
2. **Yellow** (⚠️) - Proceed with modifications (document changes)
3. **Red** (❌) - Blocked, need alternative approach

**Example Decision Log**:
```markdown
## SPIKE 1: GraphQL Subscriptions
**Status**: ⚠️ Yellow
**Findings**: Juniper subscriptions work but have memory leak on disconnect
**Decision**: Proceed with workaround (add cleanup hook) + file upstream issue
**Impact**: +0.5 days to implement cleanup

## SPIKE 2: Intake AI
**Status**: ✅ Green
**Findings**: Conversations are natural, profile quality is good (avg 4.2/5)
**Decision**: Proceed as designed
**Impact**: None
```

---

## Risk Matrix (Before Spikes)

```
High Risk, High Impact:
- GraphQL Subscriptions      [SPIKE 1]
- Intake AI Quality           [SPIKE 2]
- Content Hash Accuracy       [SPIKE 3]

Medium Risk, High Impact:
- Expo Push Reliability       [SPIKE 4]
- Need Extraction Quality     [SPIKE 5]
- pgvector Performance        [SPIKE 6]

Low Risk, Nice to Validate:
- Redis Multi-Server          [SPIKE 7]
- Batch Mutation Performance  [SPIKE 8]
```

---

## Success Metrics

At end of spike week:
- **8/8 spikes green** → Full speed ahead (15-17 day timeline holds)
- **6-7 green, 1-2 yellow** → Proceed with minor adjustments (+1-2 days)
- **5 green, 3 yellow** → Significant rework needed (+3-5 days)
- **Any red** → Re-architect affected areas (+1-2 weeks)

---

## Spike Documentation Template

```markdown
# SPIKE N: [Name]

## Goal
[What we're trying to prove]

## Time Box
[Maximum time to spend]

## Setup
[Prerequisites, test environment]

## Tasks
- [ ] Task 1
- [ ] Task 2
- [ ] Task 3

## Results

### What Worked
- Finding 1
- Finding 2

### What Didn't Work
- Issue 1
- Issue 2

### Surprises
- Unexpected finding 1

## Decision
**Status**: ✅ Green / ⚠️ Yellow / ❌ Red

**Rationale**: [Why this decision]

**Impact**: [Timeline/architecture changes]

## Code Artifacts
[Link to spike code directory]

## Next Steps
[What to do next based on findings]
```

---

## Ready to Start?

Recommended order:
1. **SPIKE 2 (Intake AI)** - Highest uncertainty, defines user experience
2. **SPIKE 1 (GraphQL Subscriptions)** - Blocks chat feature entirely
3. **SPIKE 3 (Content Hash)** - Defines sync strategy
4. **SPIKE 5 (Need Extraction)** - Validates core value prop
5. Others in parallel (can be done independently)

Which spike should we start with?
