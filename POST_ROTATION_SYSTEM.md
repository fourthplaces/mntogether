# Post Rotation System: Engagement-Based Fair Visibility

## Overview

This system ensures that all published posts receive fair visibility over time, preventing newer posts from permanently overshadowing older, under-engaged content. It implements a **round-robin engagement tracking** algorithm that prioritizes posts with lower view counts and longer time since last display.

## Problem Solved

Without rotation:
- Newest posts always appear first
- Older posts with few views get buried and never resurface
- Some opportunities never reach their intended audience
- Users see the same popular posts repeatedly

With rotation:
- All posts cycle through the feed over time
- Under-engaged posts get periodic visibility boosts
- Fair exposure for all content throughout its lifetime
- Fresh content mix for repeat visitors

## Architecture

### Database Schema

**New Column**: `last_displayed_at TIMESTAMP WITH TIME ZONE`
- Added to `posts` table via migration `000023`
- Tracks when a post was last fetched in a published posts query
- Defaults to NULL for new posts (highest priority)
- Updated asynchronously when posts are served to users

**Index**: `idx_posts_rotation`
```sql
CREATE INDEX idx_posts_rotation ON posts(view_count ASC, last_displayed_at ASC NULLS FIRST)
WHERE status = 'published';
```

### Rotation Algorithm

Posts are sorted by two factors in priority order:

1. **View Count (Ascending)**
   - Posts with fewer views shown first
   - Ensures under-engaged content gets exposure
   - Primary fairness metric

2. **Last Displayed At (Ascending, NULLS FIRST)**
   - Among posts with similar view counts, show oldest-displayed first
   - Never-displayed posts (NULL) get highest priority
   - Prevents starvation of specific posts

**SQL Query**:
```sql
SELECT * FROM posts
WHERE status = 'published'
  AND (expires_at IS NULL OR expires_at > NOW())
ORDER BY
    view_count ASC,
    last_displayed_at ASC NULLS FIRST
LIMIT 50
```

### Update Flow

```
1. User requests published posts
   ↓
2. Server fetches posts sorted by rotation algorithm
   ↓
3. Posts are returned to user immediately
   ↓
4. Background task updates last_displayed_at for all returned posts
   ↓
5. Next query considers these updated timestamps for fair rotation
```

**Key Design Decision**: Update happens *after* query, not during
- Avoids transaction overhead in read path
- Allows fast query responses
- Accurately reflects when posts were actually delivered to users
- Failures don't block responses (logged as warnings)

## Code Locations

### Database
- **Migration**: `packages/server/migrations/000023_add_last_displayed_at_to_posts.sql`
- Creates column, index, and documentation

### Models (Data Layer)
- **File**: `packages/server/src/domains/organization/models/post.rs`
- **Struct**: Updated `Post` with `last_displayed_at` field
- **Query**: `Post::find_published()` - Implements rotation sorting
- **Update**: `Post::mark_displayed()` - Batch updates timestamps

### Edges (Business Logic)
- **File**: `packages/server/src/domains/organization/edges/post_edges.rs`
- **Function**: `query_published_posts()`
  - Fetches posts using rotation algorithm
  - Spawns background task to update `last_displayed_at`
  - Returns posts immediately without waiting

## How It Works: Step by Step

### Initial State
```
Post A: view_count=0, last_displayed_at=NULL
Post B: view_count=0, last_displayed_at=NULL
Post C: view_count=0, last_displayed_at=NULL
```
**Query Result**: A, B, C (or any order, all have same priority)

### After User 1 Views Feed
```
Post A: view_count=0, last_displayed_at=2024-01-28 10:00:00
Post B: view_count=0, last_displayed_at=2024-01-28 10:00:00
Post C: view_count=0, last_displayed_at=2024-01-28 10:00:00
```
All posts shown, all timestamps updated.

### User 1 Views Post A (Analytics Tracked)
```
Post A: view_count=1, last_displayed_at=2024-01-28 10:00:00
Post B: view_count=0, last_displayed_at=2024-01-28 10:00:00
Post C: view_count=0, last_displayed_at=2024-01-28 10:00:00
```
Post A now has higher view count.

### User 2 Views Feed (1 Hour Later)
**Query Result**: B, C, A
- B and C have lower view_count (0 vs 1)
- B and C shown first
- Their timestamps update to 11:00:00
- A remains at 10:00:00

```
Post A: view_count=1, last_displayed_at=2024-01-28 10:00:00
Post B: view_count=0, last_displayed_at=2024-01-28 11:00:00
Post C: view_count=0, last_displayed_at=2024-01-28 11:00:00
```

### User 2 Views Post B
```
Post A: view_count=1, last_displayed_at=2024-01-28 10:00:00
Post B: view_count=1, last_displayed_at=2024-01-28 11:00:00
Post C: view_count=0, last_displayed_at=2024-01-28 11:00:00
```

### User 3 Views Feed (2 Hours Later)
**Query Result**: C, A, B
- C has lowest view_count (0)
- Between A and B (both view_count=1), A shown first (older timestamp)
- C shown first overall

**Pattern**: Post C, despite being older, gets prominent placement because it has the fewest views.

## Benefits

1. **Fair Exposure**: Every post gets multiple chances to be seen
2. **Discovery**: Users encounter variety, not just popular content
3. **Opportunity Equity**: Under-engaged opportunities don't disappear
4. **Freshness**: Repeat visitors see different content mixes
5. **Natural Decay**: Posts with high engagement naturally appear less often
6. **No Gaming**: Algorithm is deterministic and based on actual user behavior

## Edge Cases

### New Post Added
- `last_displayed_at` = NULL
- `view_count` = 0
- Gets highest priority in next query
- Quickly establishes baseline engagement

### Post Expires
- Removed from rotation via `WHERE` clause
- No cleanup needed for timestamps

### High Traffic
- Timestamps update frequently
- Rotation happens naturally as posts cycle
- Index ensures efficient sorting even with many posts

### Low Traffic
- Posts may appear in same order multiple times
- Eventually timestamps diverge as users engage
- System still ensures fair distribution over time

## Performance Considerations

### Query Performance
- Composite index `(view_count, last_displayed_at)` optimized for sort
- WHERE filter reduces candidate set
- NULLS FIRST handled efficiently by index

### Update Performance
- Background task (non-blocking)
- Batch update using `ANY($1)` for all post IDs
- Failures logged but don't affect user experience
- Single UPDATE statement per fetch batch

### Scalability
- Query complexity: O(log n) with index
- Update complexity: O(posts_returned)
- No per-post overhead
- Handles thousands of posts efficiently

## Monitoring & Debugging

### Check Rotation Distribution
```sql
SELECT
    view_count,
    COUNT(*) as posts_count,
    MIN(last_displayed_at) as oldest_display,
    MAX(last_displayed_at) as newest_display
FROM posts
WHERE status = 'published'
GROUP BY view_count
ORDER BY view_count;
```

### Find Stale Posts
```sql
SELECT id, view_count, last_displayed_at
FROM posts
WHERE status = 'published'
  AND (last_displayed_at IS NULL OR last_displayed_at < NOW() - INTERVAL '7 days')
ORDER BY view_count ASC, last_displayed_at ASC NULLS FIRST
LIMIT 10;
```

### Verify Update Success
```sql
SELECT
    COUNT(*) FILTER (WHERE last_displayed_at IS NULL) as never_shown,
    COUNT(*) FILTER (WHERE last_displayed_at > NOW() - INTERVAL '1 hour') as shown_recently,
    COUNT(*) as total_published
FROM posts
WHERE status = 'published';
```

## Future Enhancements

Possible additions (not currently implemented):

1. **Time Decay Factor**: Add weight for post age
2. **Urgency Boost**: Priority multiplier for urgent posts
3. **Location Awareness**: Rotate based on user location
4. **A/B Testing**: Compare rotation strategies
5. **Manual Pinning**: Allow admins to pin critical posts

## Testing

### Manual Testing
1. Create multiple posts
2. Query published posts endpoint
3. Verify posts with 0 views appear first
4. Track a view for one post
5. Query again - post with 1 view should appear later
6. Check `last_displayed_at` timestamps are updating

### Load Testing
```bash
# Generate load to verify rotation under traffic
ab -n 1000 -c 10 http://localhost:4000/graphql \
  -p query_posts.json -T "application/json"
```

## Documentation Files

This feature is documented in:
- This file: High-level design and algorithm
- Migration: Schema changes and rationale
- Code comments: Implementation details and design decisions
- Post model: Algorithm documentation in function docs
- Post edges: Update flow documentation

## Questions?

For more details:
- See inline code comments in affected files
- Check migration file for database design rationale
- Review test cases (when implemented)
- Ask the team about observed behavior
