# Admin App — Design System

Built on **shadcn/ui** (Base UI primitives + Tailwind v4). One UI library, one theming system.

---

## Rule #1: Use shadcn components

Before writing custom HTML for any interactive or structural UI element, check if a shadcn component exists for it. If it does, use it. If we don't have it installed yet, install it (`npx shadcn@latest add <component>`).

**Never hand-roll** what shadcn already provides:
- Tabs instead of custom button groups for filters
- Table instead of manual `<table>` markup
- Alert instead of `<div className="bg-red-50 border border-red-200 ...">`
- Select instead of native `<select>`
- Separator instead of `<div className="border-t ...">`
- Dialog instead of custom modals
- Badge instead of custom pill/tag spans

### Installed components (`components/ui/`)

| Component | Base UI primitive | Notes |
|-----------|------------------|-------|
| Accordion | Accordion | Expandable sections |
| Alert | — | Status banners (success/error/warning/info) |
| Badge | useRender | Tags, status labels. Supports `render` prop and dynamic `color` prop |
| Breadcrumb | useRender | Navigation breadcrumbs |
| Button | Button | Primary interactive element. Use `render` prop for links |
| Calendar | react-day-picker | Date picker (used inside Popover) |
| Card | — | Container with border/shadow variants |
| Checkbox | Checkbox | Form checkboxes |
| Collapsible | Collapsible | Expandable content sections |
| Command | cmdk | Command palette (combobox) |
| ContextMenu | Menu | Right-click context menus |
| Dialog | Dialog | Modal overlay with backdrop, escape-to-close |
| DropdownMenu | Menu | Action menus, context menus |
| Field | — | Form field wrapper with label + description |
| HoverCard | Popover | Hover-triggered content cards |
| Input | — | Text inputs with error state |
| Label | — | Form labels (pure HTML) |
| Popover | Popover | Floating content (used by Calendar, Select) |
| Progress | Progress | Progress bars |
| RadioGroup | RadioGroup | Radio button groups |
| ScrollArea | ScrollArea | Custom scrollbar areas |
| Select | Select | Dropdown select. `onValueChange` passes `string \| null` |
| Separator | Separator | Horizontal/vertical dividers |
| Sheet | Dialog | Slide-out side panels |
| Skeleton | — | Loading placeholder animations |
| Switch | Switch | Boolean toggle (use for settings, NOT filters) |
| Table | — | Data tables with header/body/row/cell |
| Tabs | Tabs | Filter groups, view mode switches |
| Textarea | — | Multi-line text input with error state |
| Toggle | Toggle | Pressable toggle buttons |
| ToggleGroup | ToggleGroup | Grouped toggle buttons (uses array values) |
| Tooltip | Tooltip | Hover tooltips |

---

## Rule #2: Use token colors, never hardcoded values

All borders, backgrounds, and text colors must use the semantic token classes — never Tailwind color scales directly.

### Common mistakes

```tsx
// BAD — hardcoded Tailwind color scale
className="border-stone-200"
className="border-gray-300"
className="bg-stone-50"
className="text-stone-500"

// GOOD — semantic tokens
className="border-border"
className="bg-muted"
className="text-muted-foreground"
```

### Watch for 1px black borders

shadcn components default to `border-border` which resolves to our warm `#E8DED2`. But when adding `border` to a raw HTML element without a color class, Tailwind renders a **1px solid black** border (or whatever the browser default is). Always pair `border` with a color:

```tsx
// BAD — gets a black border
className="border rounded-lg"

// GOOD — explicit token color
className="border border-border rounded-lg"
```

This also applies inside shadcn components when overriding styles. If you see a harsh black or gray border that doesn't match, it's this bug.

### Semantic color tokens

These are defined in `globals.css` and mapped to our warm-earth palette:

| Token class | CSS variable | Hex value | Use for |
|-------------|-------------|-----------|---------|
| `bg-background` | `--background` | `#FDFCFA` | Page background |
| `text-foreground` | `--foreground` | `#3D3D3D` | Primary text |
| `bg-card` | `--card` | `#FFFFFF` | Card/panel backgrounds |
| `text-card-foreground` | `--card-foreground` | `#3D3D3D` | Text on cards |
| `bg-muted` | `--muted` | `#F5F1E8` | Subtle backgrounds, hover states |
| `text-muted-foreground` | `--muted-foreground` | `#7D7D7D` | Secondary text, timestamps |
| `bg-accent` | `--accent` | `#F5F1E8` | Active/selected states |
| `text-accent-foreground` | `--accent-foreground` | `#3D3D3D` | Text on accent |
| `bg-primary` | `--primary` | `#3D3D3D` | Primary buttons |
| `text-primary-foreground` | `--primary-foreground` | `#FFFFFF` | Text on primary |
| `bg-secondary` | `--secondary` | `#F5F1E8` | Secondary buttons, table headers |
| `bg-destructive` | `--destructive` | `#F43F5E` | Danger actions |
| `border-border` | `--border` | `#E8DED2` | All borders and dividers |
| `border-input` | `--input` | `#E8DED2` | Input borders |
| `ring-ring` | `--ring` | `#C4B8A0` | Focus rings |

### Admin-specific tokens (from `themes/warm-earth.css`)

| Token class | Purpose |
|-------------|---------|
| `bg-admin-accent` / `hover:bg-admin-accent-hover` | Admin primary buttons (amber) |
| `ring-admin-focus-ring` | Admin focus ring |
| `bg-success-bg` / `text-success-text` | Success states |
| `bg-danger-bg` / `text-danger-text` | Error states |
| `bg-warning-bg` / `text-warning-text` | Warning states |
| `bg-info-bg` / `text-info-text` | Info states |

---

## Rule #3: Follow shadcn patterns

### Table pattern

```tsx
<div className="rounded-lg border border-border overflow-hidden bg-card">
  <Table>
    <TableHeader>
      <TableRow>
        <TableHead className="pl-6">Name</TableHead>
        <TableHead>Status</TableHead>
        <TableHead className="w-10" />
      </TableRow>
    </TableHeader>
    <TableBody>
      {items.map((item) => (
        <TableRow key={item.id} className="cursor-pointer">
          <TableCell className="pl-6">{item.name}</TableCell>
          ...
        </TableRow>
      ))}
    </TableBody>
  </Table>
</div>
```

Key points:
- Wrapper div provides `rounded-lg border border-border overflow-hidden bg-card`
- No `shadow-sm` on list tables (save elevation for cards)
- First column uses `pl-6` for left inset
- Clickable rows get `className="cursor-pointer"` on TableRow
- TableHead/TableCell provide their own padding (`px-4`)

### Filter pattern (Tabs, not buttons)

```tsx
<div className="flex items-center gap-3 mb-4">
  <Tabs value={filter} onValueChange={setFilter}>
    <TabsList>
      <TabsTrigger value="all">All</TabsTrigger>
      <TabsTrigger value="active">Active</TabsTrigger>
    </TabsList>
  </Tabs>
  <input className="h-9 flex-1 px-3 border border-border rounded-lg text-sm bg-background ..." />
</div>
```

- Use `Tabs` for list page filters, view mode toggles, and tab groups
- Use `Switch` only for boolean settings (not filters on list pages)
- Search inputs sit alongside Tabs in a flex row, matching `h-9` height

### Alert pattern

```tsx
<Alert variant="error">
  Something went wrong. Please try again.
</Alert>
```

Never hand-roll `bg-red-50 border border-red-200 text-red-700` divs. Use the Alert component.

---

## Architecture

```
components/ui/         ← shadcn primitives (Base UI + Tailwind)
components/admin/      ← admin-specific composites (AdminSidebar, TagsSection, etc.)
app/globals.css        ← Layer 1: shadcn semantic variables
app/themes/warm-earth.css ← Layer 3: domain-specific tokens
lib/utils.ts           ← cn() helper (clsx + tailwind-merge)
```

### Stack

| Layer | Technology |
|-------|-----------|
| Primitives | Base UI (`@base-ui/react` v1.2+) |
| Components | shadcn/ui (wraps Base UI with Tailwind styling) |
| Styling | Tailwind CSS v4 with `@theme` token bridge |
| Variants | class-variance-authority (CVA) |
| Class merging | clsx + tailwind-merge via `cn()` |
| Icons | lucide-react |

### `cn()` utility

```tsx
import { cn } from "@/lib/utils";

cn("base-class", isActive && "active-class", size === "lg" && "text-lg")
```

---

## Conventions

1. **shadcn first.** Check installed components before writing HTML. Install new ones if needed.
2. **Token colors only.** No `stone-*`, `gray-*`, `slate-*` for structural UI. Semantic tokens everywhere.
3. **No `border` without a color.** Always pair `border` with `border-border` (or another token).
4. **No `shadow-sm` on list tables.** Tables use `border border-border`, not shadows.
5. **Tabs for filters, Switch for settings.** Toggles on list pages should use Tabs, not Switch.
6. **Lucide icons, not inline SVGs.** Import from `lucide-react` instead of pasting `<svg>` markup.
7. **`cn()` for conditional classes.** Never string-concatenate Tailwind classes manually.
