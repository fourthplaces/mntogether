# Semantic Search for Organizations

Production-ready AI-powered organization matching using vector embeddings.

## Overview

The semantic search system enables users to find relevant organizations by describing their needs in natural language. Instead of keyword matching, it uses OpenAI embeddings to understand semantic meaning and find the best matches.

**Example:**
```
Query: "I need immigration legal help and speak Spanish"
Results:
- Immigration Law Center (95% match)
- Legal Aid Services (88% match)
- Community Legal Clinic (82% match)
```

## Architecture

```
User Query
    ↓
Generate Embedding (OpenAI API)
    ↓
Vector Similarity Search (PostgreSQL + pgvector)
    ↓
Ranked Results (with similarity scores)
```

## Setup

### 1. Database Migration

Apply the embeddings migration:

```bash
sqlx migrate run
```

This adds:
- `embedding` column (vector[1536]) to `organizations` table
- `summary` column for rich organization descriptions
- Vector similarity index (ivfflat)
- PostgreSQL function `search_organizations_by_similarity()`

### 2. Generate Embeddings

For existing organizations:

```bash
cargo run --bin generate_embeddings
```

This will:
- Find all organizations without embeddings
- Generate embeddings for `description + summary`
- Update the database
- Rate limit to avoid API throttling (100ms between calls)
- Retry failed calls (max 3 attempts with exponential backoff)

### 3. GraphQL API

Query organizations using semantic search:

```graphql
query SearchOrganizations {
  searchOrganizationsSemantic(
    query: "I need immigration legal help in Spanish"
    limit: 10
  ) {
    organization {
      id
      name
      description
      summary
      website
      phone
    }
    similarityScore
  }
}
```

## Configuration

### Similarity Threshold

Default: `0.7` (70% similarity)

Adjust in code:
```rust
let config = AIMatchingConfig {
    similarity_threshold: 0.8, // More strict
    result_limit: 5,
    max_retries: 3,
};

let service = AIMatchingService::with_config(openai_client, config);
```

### Result Limit

Default: `10` organizations

Override per-query:
```graphql
searchOrganizationsSemantic(query: "...", limit: 20)
```

## Production Considerations

### Rate Limiting

Built-in rate limiting:
- 100ms delay between API calls
- Prevents hitting OpenAI rate limits
- Configurable per environment

### Error Handling

Automatic retry logic:
- Max 3 retry attempts
- Exponential backoff (1s, 2s, 3s)
- Detailed error logging

### Monitoring

Key metrics to track:
- Embedding generation time
- Search query latency
- API error rate
- Similarity score distribution

Example logging:
```
[INFO] Finding relevant organizations using semantic search
[DEBUG] Generated embedding for user query (1536 dims)
[INFO] Found 5 relevant organizations
```

### Cost Optimization

OpenAI API costs:
- Embedding generation: ~$0.0001 per request
- Cache embeddings in database
- Only regenerate when organization details change

### Data Quality

For best results:
1. **Rich Descriptions**: Include comprehensive service info in `summary`
2. **Regular Updates**: Regenerate embeddings when organizations change
3. **Tag Integration**: Combine semantic search with tag filtering

Example good summary:
```
Immigration law services specializing in family reunification,
asylum cases, and deportation defense. Spanish and Somali
language support available. Free consultations for low-income
families. Evening and weekend hours.
```

## API Reference

### AIMatchingService

```rust
pub struct AIMatchingService {
    openai_client: OpenAIClient,
    config: AIMatchingConfig,
}
```

#### Methods

##### `find_relevant_organizations`
```rust
pub async fn find_relevant_organizations(
    &self,
    user_query: String,
    pool: &PgPool,
) -> Result<Vec<(Organization, f32)>>
```

Find organizations matching the user query with default config.

##### `find_relevant_organizations_with_config`
```rust
pub async fn find_relevant_organizations_with_config(
    &self,
    user_query: String,
    similarity_threshold: f32,
    limit: i32,
    pool: &PgPool,
) -> Result<Vec<(Organization, f32)>>
```

Find organizations with custom threshold and limit.

##### `generate_organization_embedding`
```rust
pub async fn generate_organization_embedding(
    &self,
    org: &Organization
) -> Result<Vec<f32>>
```

Generate embedding for a single organization.

##### `update_missing_embeddings`
```rust
pub async fn update_missing_embeddings(
    &self,
    pool: &PgPool
) -> Result<usize>
```

Batch update all organizations missing embeddings.

### GraphQL Queries

#### `searchOrganizationsSemantic`

```graphql
searchOrganizationsSemantic(
  query: String!
  limit: Int
): [OrganizationMatchData!]!
```

**Arguments:**
- `query`: User's natural language query
- `limit`: Max results (default: 10)

**Returns:**
```graphql
type OrganizationMatchData {
  organization: OrganizationData!
  similarityScore: Float!
}
```

## Testing

### Unit Tests

```bash
cargo test ai_matching
```

### Integration Tests

Requires running PostgreSQL with pgvector:

```bash
cargo test --ignored test_find_relevant_organizations
```

### Manual Testing

1. Start the server
2. Use GraphQL Playground: `http://localhost:8080/graphql`
3. Run test queries:

```graphql
{
  searchOrganizationsSemantic(
    query: "legal help for immigrants"
    limit: 5
  ) {
    organization {
      name
    }
    similarityScore
  }
}
```

## Troubleshooting

### No Results Found

**Symptom**: Search returns empty array

**Solutions:**
1. Check if organizations have embeddings: `SELECT COUNT(*) FROM organizations WHERE embedding IS NOT NULL`
2. Run `cargo run --bin generate_embeddings`
3. Lower similarity threshold: `similarity_threshold: 0.5`

### Slow Queries

**Symptom**: Search takes >1 second

**Solutions:**
1. Verify vector index exists: `\d+ organizations` in psql
2. Increase index lists: `CREATE INDEX ... WITH (lists = 200)`
3. Add query timeout in application

### API Rate Limits

**Symptom**: `429 Too Many Requests` from OpenAI

**Solutions:**
1. Increase delay between calls (default: 100ms)
2. Use exponential backoff (already implemented)
3. Cache embeddings aggressively
4. Consider batch embedding generation during off-peak hours

## Future Enhancements

- [ ] Multi-language support (embed in multiple languages)
- [ ] Hybrid search (combine semantic + keyword + tags)
- [ ] Query understanding (extract intent, location, language preference)
- [ ] Result ranking (blend similarity with recency, capacity status)
- [ ] A/B testing framework for threshold tuning
- [ ] Real-time embedding updates via CDC
