# Plan: MVP Map Page — Active Posts Across Minnesota

## Context

The `heat_map` domain exists but is note-severity-weighted and not what we want. The goal is a simple, lightweight map page showing all active posts with locations as dots on an OpenStreetMap view of Minnesota. Zip-code clusters get low-opacity large circles. No county/edition scoping — statewide, all live posts.

## Approach

**Direct REST endpoint** (not GraphQL) — the map needs a flat list of (id, title, lat, lng, postType) that no existing query returns. A new `/Posts/map_points` public handler calls the Rust server directly from a Next.js server component via the existing `serverCall` pattern. No new npm dependencies — Leaflet loaded via CDN `next/script`.

## Changes

### 1. Rust: Model methods

**`packages/server/src/domains/posts/models/post.rs`**

Add two structs + methods:

- `MapPoint` — (id, title, post_type, latitude, longitude, location_text, postal_code)
- `MapCluster` — (postal_code, city, latitude, longitude, post_count)
- `Post::find_map_points(pool)` — JOIN `post_locations → locations` (same pattern as `find_near_zip`), filter `status = 'active'`, apply `SCHEDULE_ACTIVE_FILTER`
- `Post::find_map_clusters(pool)` — same join + GROUP BY postal_code with COUNT

### 2. Rust: HTTP handler

**`packages/server/src/api/routes/posts.rs`**

- Add `map_points` handler (no auth, like `public_list`)
- Returns `{ points: [...], clusters: [...] }`
- Register route: `.route("/Posts/map_points", post(map_points))`

### 3. Web-app: Map page

**`packages/web-app/app/(app)/map/page.tsx`** (new — server component)

- Fetches data via `serverCall<MapData>("Posts/map_points")`
- Renders heading, stats line, legend, passes data to `MapClient`

**`packages/web-app/app/(app)/map/MapClient.tsx`** (new — client component)

- Loads Leaflet 1.9.4 CSS + JS via CDN (`next/script`, `afterInteractive`)
- Initializes map centered on MN (46.3, -94.3, zoom 7)
- OpenStreetMap tiles (free, no API key)
- `L.circle` for zip clusters — radius scaled by post_count, fill opacity ~0.12
- `L.circleMarker` for individual posts — radius 5, colored by postType, popup with title + link to `/posts/{id}`

**`packages/web-app/app/(app)/map/map.css`** (new — minimal)

- Legend layout (flex-wrap dot + label items)

### 4. No migration needed

Reads existing tables (`posts`, `post_locations`, `locations`, `zip_codes`). No schema changes.

## Files

| File | Change |
|---|---|
| `packages/server/src/domains/posts/models/post.rs` | Add `MapPoint`, `MapCluster` structs + query methods |
| `packages/server/src/api/routes/posts.rs` | Add `map_points` handler + route registration |
| `packages/web-app/app/(app)/map/page.tsx` | New server component — data fetch + page layout |
| `packages/web-app/app/(app)/map/MapClient.tsx` | New client component — Leaflet map rendering |
| `packages/web-app/app/(app)/map/map.css` | Legend styles |

## Post-type color scheme

| Type | Color |
|---|---|
| story | #2563eb (blue) |
| notice | #dc2626 (red) |
| exchange | #16a34a (green) |
| event | #9333ea (purple) |
| spotlight | #ea580c (orange) |
| reference | #6b7280 (gray) |

## Verification

1. Rebuild server: `docker compose up -d --build server`
2. Curl test: `curl -X POST http://localhost:9080/Posts/map_points -H 'Content-Type: application/json' -d '{}'`
3. Visit `http://localhost:3001/map` — should see MN map with dots and cluster circles
4. Click a dot → popup with title and link to post detail page
5. Cluster circles should be large, low opacity, labeled with city + post count
