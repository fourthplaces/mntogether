---
date: 2026-02-11
topic: heat-map
---

# Heat Map: Location + Urgency Visualization

## What We're Building

A content-agnostic heat map that shows where help is needed across the service area. The abstraction pairs location + urgency signals from any entity type, computes weighted points, and stores periodic snapshots that both community members and coordinators can consume — with different views on the same underlying data.

## Key Decisions

- **Deprecate post-level `urgency`**: Note severity becomes the canonical urgency signal. Notes are already polymorphic (attached to anything via `noteables`), so urgency becomes a cross-cutting concern rather than a post-specific field.

- **`locationables` polymorphic table**: Mirrors the `noteables` pattern. Any entity can have locations the same way any entity can have notes. These two traits (`locationable` + `noteable`) are composable — anything with both can participate in the heat map.

- **Composable, pluggable scoring**: The heat map abstraction only knows about `(lat, lng, weight, entity_type, entity_id)` tuples. It does not know about posts, notes, or any specific entity type. The scoring function is pluggable — different layers or views can compute weight differently.

- **Max severity wins**: When an entity has multiple notes, the highest severity determines its weight. An entity with both `info` and `urgent` notes is weighted as `urgent`.

- **Aggregate per entity, then fan out to locations**: To avoid the cross-product problem (3 locations x 2 notes = 6 rows), notes are aggregated into a single weight per entity first, then distributed to each of that entity's locations.

- **Backend-generated via Restate**: A periodic snapshot workflow computes the heat map on a schedule and stores the result. Consumers read the latest snapshot.

- **`heat_map_points` table**: Snapshot is stored in a dedicated relational table, rebuilt each run (truncate + insert or generation column swap). Queryable, indexable, filterable.

- **Provenance retained**: Each tuple carries `entity_type` and `entity_id` so consumers can filter by type/category and drill down to source entities.

- **No-location urgency is invisible**: If an entity has urgent notes but no location, it won't appear on the heat map. This is acceptable and documented behavior.

## Two Consumer Views

- **Community members**: "Where is help needed?" — high urgency + low capacity = go here
- **Coordinators/admins**: "Where are the gaps?" — cluster analysis of unmet needs, resource allocation

Same data, different rendering/filtering on the client side.

## Data Flow

```
Scheduled Restate Workflow (periodic)
  → Query: JOIN locationables + noteables
      WHERE notes are active (not expired)
      GROUP BY entity → max(severity) as weight
  → Fan out to entity locations via locationables
  → Produce (lat, lng, weight, entity_type, entity_id) tuples
  → Truncate + insert into heat_map_points
  → Consumers read latest snapshot
```

## Data Model Additions

### `locationables` (new — mirrors `noteables`)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID | PK |
| location_id | UUID | FK → locations |
| locatable_type | TEXT | e.g., 'post', 'organization' |
| locatable_id | UUID | polymorphic FK |
| is_primary | BOOL | primary location flag |
| notes | TEXT | location-specific notes |
| added_at | TIMESTAMPTZ | |

Indexes: `(locatable_type, locatable_id)`, `(location_id)`

### `heat_map_points` (new — snapshot output)

| Column | Type | Notes |
|--------|------|-------|
| id | UUID | PK |
| latitude | FLOAT8 | |
| longitude | FLOAT8 | |
| weight | FLOAT8 | computed score |
| entity_type | TEXT | provenance |
| entity_id | UUID | provenance |
| generated_at | TIMESTAMPTZ | snapshot timestamp |

Indexes: `(generated_at)`, `(entity_type)`, spatial index on `(latitude, longitude)` if needed

## Migration Path for `post_locations` → `locationables`

The existing `post_locations` table is post-specific. With `locationables`, posts would use the generic table instead. Migration strategy:

1. Create `locationables` table
2. Migrate existing `post_locations` data into `locationables` with `locatable_type = 'post'`
3. Update post location queries to use `locationables`
4. Deprecate `post_locations`

## Scale

- Thousands of entities — periodic snapshot query is trivially fast at this scale
- Composite indexes on polymorphic joins ensure the query stays efficient

## Open Questions

- Snapshot frequency — every hour? Every 15 minutes? Configurable?
- Weight values — what numeric values map to `info`, `notice`, `urgent`? (e.g., 1, 5, 10?)
- Should `capacity_status` factor into weight computation as a second signal, or keep it pure urgency for v1?
- Client rendering approach — outside scope of this brainstorm, but will need to be decided

## Next Steps

`/workflows:plan` for implementation details
