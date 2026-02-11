---
date: 2026-02-11
topic: zip-code-proximity-filtering
---

# Zip Code Proximity Filtering for Posts

## What We're Building

Composable zip code proximity filtering for posts, exposed through Restate services and consumed by both admin and public web UIs. Users enter a zip code and radius to find posts within a geographic area, with results sorted by distance.

## Why This Approach

The backend infrastructure already exists: `locations` table with postal codes and coordinates, `post_locations` join table, `zip_codes` reference table (seeded with MN data), `haversine_distance()` SQL function, and a standalone `find_near_zip` query. Rather than building a parallel system, we compose zip/radius as additional filter dimensions into the existing post listing queries.

## Key Decisions

- **Proximity-based, not exact match**: Zip code + radius in miles. A service 1 mile away in a different zip still shows up. Uses existing `haversine_distance` function.
- **Composed into existing queries**: Zip/radius are optional parameters on the main post listing query, not a separate endpoint. Combinable with status, post_type, category, and other filters.
- **Clean return types**: When zip filter is active, response uses a `PostWithDistance` type that includes `distance_miles`. When no zip filter, response uses regular `Post` type. No nullable distance field bolted onto every post.
- **Leave `find_near_zip` alone**: The existing standalone method stays for the `NearbySearchRequest` Restate handler. No refactor for refactor's sake.
- **Default radius**: 25 miles when zip is provided without explicit radius.
- **Validate zip codes**: Check against `zip_codes` table, return error for unknown zips.

## What Needs to Change

### 1. Model Layer
Extend the post listing query to accept optional `zip_code` + `radius_miles` parameters. When present:
- Join `post_locations -> locations -> zip_codes`
- Apply haversine distance filter
- Return `PostWithDistance` (includes `distance_miles`, `zip_code`, `location_city`)
- Sort by proximity

When absent: return regular `Post`, existing sort behavior.

### 2. Restate Service
Update the post listing handler request type to include optional `zip_code: Option<String>` and `radius_miles: Option<f64>`. Delegate to the composed model query. Response type branches based on whether zip filter was applied.

### 3. Admin Web UI
- Add zip code text input to the posts filter bar
- Add radius dropdown (5, 10, 25, 50 miles)
- When active: show distance column, sort by proximity
- When cleared: revert to default list behavior

### 4. Public Web UI (later)
Same filtering capability, surfaced through the public-facing search/directory.

## Existing Infrastructure (no changes needed)

- `locations` table with `postal_code`, `latitude`, `longitude`
- `post_locations` join table (posts <-> locations, many-to-many)
- `zip_codes` reference table (MN seeded, 54k+ entries)
- `haversine_distance()` SQL function
- `PostWithDistance` struct
- `Post::find_near_zip()` standalone method
- `NearbySearchRequest` Restate handler

## Next Steps

-> `/workflows:plan` for implementation details
