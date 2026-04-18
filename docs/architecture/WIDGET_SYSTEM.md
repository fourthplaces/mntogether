# Widget System

> **Pre-migration design doc.** Written when the plan routed all backend
> work through Restate. Restate was removed on 2026-03-17 (see
> `ARCHITECTURE_DECISIONS.md` Decision 4). References below to
> "Restate handlers" correspond to Axum HTTP handlers in
> `src/api/routes/widgets.rs` in the current codebase. Architectural
> intent is preserved.

## Overview

Widgets are non-post content elements placed in broadsheet edition rows. They support decorative and informational content like section headers, weather forecasts, and hotline/resource bars.

## Architecture

### Database

```sql
CREATE TABLE edition_widgets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    edition_row_id  UUID NOT NULL REFERENCES edition_rows(id) ON DELETE CASCADE,
    widget_type     TEXT NOT NULL,
    slot_index      INT NOT NULL DEFAULT 0,
    config          JSONB NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

JSONB is used for `config` because each widget type has a genuinely different schema shape. This is one of the approved JSONB use cases per project conventions.

### Widget Types

| Type | Config Shape | Visual |
|------|-------------|--------|
| `section_header` | `{ title: string, subtitle?: string }` | Full-width heading divider |
| `weather` | `{ location_id?: string }` | Weather forecast card (defaults to county) |
| `hotline_bar` | `{ lines: [{ label, phone, description? }] }` | Resource phone numbers bar |

### Data Flow

```
Frontend (JSON.stringify config) → GraphQL mutation → Restate handler → Rust model (serde_json::Value) → PostgreSQL JSONB
PostgreSQL JSONB → Rust model → Restate response (serde_json to string) → GraphQL resolver → Frontend (JSON.parse)
```

Config is stored as JSONB in Postgres but serialized to a JSON string in the GraphQL schema, since GraphQL lacks a native JSON scalar. The frontend handles serialization/deserialization.

## Backend

### Model (`edition_widget.rs`)

- `EditionWidget::create(edition_row_id, widget_type, slot_index, config, pool)`
- `EditionWidget::update(id, config, pool)`
- `EditionWidget::delete(id, pool)`
- `EditionWidget::find_by_row(edition_row_id, pool)`
- `EditionWidget::find_by_id(id, pool)`

### Restate Handlers

- `add_widget(AddWidgetRequest)` - Creates a new widget in a row
- `update_widget(UpdateWidgetRequest)` - Updates widget config
- `remove_widget(RemoveWidgetRequest)` - Deletes a widget

### GraphQL

```graphql
type EditionWidget {
  id: ID!
  widgetType: String!
  slotIndex: Int!
  config: String  # JSON-serialized
}

type EditionRow {
  # ... existing fields
  widgets: [EditionWidget!]!
}

type Mutation {
  addWidget(editionRowId: ID!, widgetType: String!, slotIndex: Int!, config: String!): EditionWidget!
  updateWidget(id: ID!, config: String!): EditionWidget!
  removeWidget(id: ID!): Boolean!
}
```

## Frontend

### Rendering

Widgets render above the post slot grid in each row. The `WidgetCard` component dispatches to type-specific renderers:

- `WidgetContent` - Switches on widget type for type-specific display
- `WidgetIcon` - Color-coded icon badge per type
- `parseWidgetConfig` - Safely parses JSON string config

### Insertion

The "+ Widget" dropdown in each row header offers the three widget types with sensible default configs. On selection, `addWidget` mutation fires with the default config and auto-incremented slot index.

### Widget Management

- Add: Dropdown menu in row header, creates widget with default config
- Remove: "Remove" button on each widget card
- Config editing: Not yet implemented (future: inline editing dialogs per type)

## Migration

File: `migrations/000179_create_edition_widgets.sql`

Run with `make migrate` before rebuilding the server.
