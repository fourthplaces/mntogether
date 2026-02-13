# Crawling Domain Migration Tracker

This document tracks the migration from legacy server code to the extraction library.

## Migration Status

### Crawling/Scraping

| Component | Status | Replacement | Safe to Remove? |
|-----------|--------|-------------|-----------------|
| `crawl_website()` | DEPRECATED | `ingest_website()` | ⚠️ After verifying no callers |
| `PageSnapshot` model | DEPRECATED | `extraction::CachedPage` | ⚠️ After data migration |
| `WebsiteSnapshot` model | DEPRECATED | `site_url` on `extraction_pages` | ⚠️ After data migration |
| `PageSummary` model | DEPRECATED | `extraction::Summary` | ⚠️ After data migration |
| `summarize_pages()` | DEPRECATED | `ExtractionService::ingest()` | ⚠️ After verifying no callers |
| `BaseWebScraper` trait | DEPRECATED | `extraction::Ingestor` | ⚠️ After verifying no callers |
| `SimpleScraper` | DEPRECATED | `extraction::HttpIngestor` | ⚠️ After verifying no callers |
| `FallbackScraper` | DEPRECATED | `ValidatedIngestor` + `FirecrawlIngestor` | ⚠️ After verifying no callers |
| `FirecrawlClient` | DEPRECATED | `extraction::FirecrawlIngestor` | ⚠️ After verifying no callers |

### Search

| Component | Status | Replacement | Safe to Remove? |
|-----------|--------|-------------|-----------------|
| `BaseSearchService` trait | DEPRECATED | `extraction::WebSearcher` | ⚠️ After verifying no callers |
| `TavilyClient` | DEPRECATED | `extraction::TavilyWebSearcher` | ⚠️ After verifying no callers |
| `NoopSearchService` | DEPRECATED | `extraction::MockWebSearcher` | ⚠️ After verifying no callers |

### Deleted Components

| Component | Deleted | Reason |
|-----------|---------|--------|
| `Resources` domain | ✅ DELETED | Unused parallel system to Posts |
| `ResourceId` | ✅ DELETED | Part of Resources domain |
| `BaseSearchService::SearchResult` | DEPRECATED | Use `extraction::SearchResult` |

## Database Tables

### Tables to Eventually DROP

```sql
-- WARNING: Only run after confirming no data is needed
-- DROP TABLE page_snapshots;
-- DROP TABLE website_snapshots;
-- DROP TABLE page_summaries;
```

### Tables to Keep (Extraction Library)

- `extraction_pages` - New page cache
- `extraction_summaries` - New summary cache
- `extraction_embeddings` - Vector embeddings
- `extraction_signals` - Extracted signals

## Removal Checklist

Before removing deprecated code, verify:

1. [ ] No GraphQL mutations call the deprecated functions directly
2. [ ] No scheduled tasks use the deprecated path
3. [ ] No effects trigger the deprecated event handlers
4. [ ] Production data has been migrated (if needed)
5. [ ] Tests have been updated to use new path

## GraphQL Migration

### Mutations Using Old Path

Check these mutations and update to use `ingest_website()`:

- `crawl_website` → Should call `ingest_website()` internally (already marked deprecated)

### Mutations Using New Path

These are already using the extraction library:

- `submitUrl` → `extraction::actions::submit_url()`
- `triggerExtraction` → `extraction::actions::trigger_extraction()`
- `ingestWebsite` → `crawling::actions::ingest_website()`

## Event Migration

### Old Events (WebsiteCrawled path)

```
WebsiteCrawled → PagesSummarized → PostsExtracted → PostsSynced
```

### New Events (WebsiteIngested path)

```
WebsiteIngested → (extraction library handles summarization internally)
```

## Code Removal Order

When ready to remove deprecated code, do it in this order:

1. **Remove callers first**
   - Update GraphQL mutations to not call deprecated functions
   - Remove deprecated event handlers from effects

2. **Remove function implementations**
   - Delete `crawl_website()`, `crawl_website_pages()`, etc.
   - Delete `summarize_pages()`, `hash_content()`, etc.

3. **Remove models**
   - Delete `PageSnapshot`, `WebsiteSnapshot`, `PageSummary`
   - Update any code that imports these

4. **Remove database tables**
   - Create migration to DROP tables
   - Only after verifying data is migrated or not needed

## Timeline

- **Now**: Code deprecated with `#[deprecated]` attributes and documentation
- **Next**: Verify all callers use new path in production
- **Future**: Remove deprecated code after confidence period
