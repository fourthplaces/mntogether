# MN Together — Design System

A token-based design system for the MN Together community platform. All visual styling flows from a small set of centralized tokens in `globals.css` and six primitive components in `components/ui/`.

**Design philosophy:** Utility first. Community, connection, trust. Clarity over flash.

---

## Quick Reference: Changing the Look

| Want to change... | Edit this |
|---|---|
| Site background color | `--color-surface` in `globals.css` |
| Card background | `--color-surface-raised` in `globals.css` |
| Primary text color | `--color-text-primary` in `globals.css` |
| Button color | `--color-action` and `--color-action-hover` in `globals.css` |
| Link color | `--color-link` and `--color-link-hover` in `globals.css` |
| Border color | `--color-border` in `globals.css` |
| Card corner radius | `--radius-lg` in `globals.css` |
| Card shadow | `--shadow-card` in `globals.css` |
| Pathway card colors | `--color-pathway-warm`, `--color-pathway-sage`, `--color-pathway-lavender` in `globals.css` |
| Admin accent color | `--color-admin-accent` in `globals.css` |

All changes cascade everywhere automatically.

---

## Tokens

Defined in `app/globals.css` inside the `@theme` block. Tailwind v4 reads these and generates utility classes automatically.

### Surface Colors (backgrounds)

| Token | Value | Utility Class | Purpose |
|---|---|---|---|
| `--color-surface` | `#E8E2D5` | `bg-surface` | Page background (warm sand) |
| `--color-surface-raised` | `#FFFFFF` | `bg-surface-raised` | Cards, panels, elevated containers |
| `--color-surface-subtle` | `#FDFCFA` | `bg-surface-subtle` | Input backgrounds |
| `--color-surface-muted` | `#F5F1E8` | `bg-surface-muted` | Tag backgrounds, empty states |

### Text Hierarchy

| Token | Value | Utility Class | Purpose |
|---|---|---|---|
| `--color-text-primary` | `#3D3D3D` | `text-text-primary` | Headings, primary content |
| `--color-text-body` | `#4D4D4D` | `text-text-body` | Body text |
| `--color-text-secondary` | `#5D5D5D` | `text-text-secondary` | Supporting text |
| `--color-text-muted` | `#7D7D7D` | `text-text-muted` | Timestamps, tertiary |
| `--color-text-faint` | `#B5AFA2` | `text-text-faint` | Placeholders, disabled |
| `--color-text-label` | `#A09A8D` | `text-text-label` | Uppercase section labels |
| `--color-text-on-action` | `#FFFFFF` | `text-text-on-action` | Text on dark buttons |

### Borders

| Token | Value | Utility Class | Purpose |
|---|---|---|---|
| `--color-border` | `#E8DED2` | `border-border` | Default borders, dividers |
| `--color-border-strong` | `#C4B8A0` | `border-border-strong` | Active borders, tab outlines |
| `--color-border-subtle` | `#F0EBE0` | `border-border-subtle` | Very light separators |

### Interactive Colors

| Token | Value | Utility Class | Purpose |
|---|---|---|---|
| `--color-action` | `#3D3D3D` | `bg-action` | Primary buttons |
| `--color-action-hover` | `#2D2D2D` | `hover:bg-action-hover` | Button hover |
| `--color-link` | `#8B6D3F` | `text-link` | Text links |
| `--color-link-hover` | `#6D5530` | `hover:text-link-hover` | Link hover |
| `--color-focus-ring` | `#C4B8A0` | `ring-focus-ring` | Focus ring (public) |

### Admin Accent

| Token | Value | Utility Class | Purpose |
|---|---|---|---|
| `--color-admin-accent` | `#D97706` | `bg-admin-accent` | Admin primary buttons |
| `--color-admin-accent-hover` | `#B45309` | `hover:bg-admin-accent-hover` | Admin hover |
| `--color-admin-focus-ring` | `#F59E0B` | `ring-admin-focus-ring` | Admin focus ring |

### Semantic Status Colors

| Token | Value | Utility Class | Purpose |
|---|---|---|---|
| `--color-success` | `#34D399` | `bg-success` | Success state |
| `--color-success-bg` | `#DCFCE7` | `bg-success-bg` | Success background |
| `--color-success-text` | `#166534` | `text-success-text` | Success text |
| `--color-danger` | `#FB7185` | `bg-danger` | Danger state |
| `--color-danger-bg` | `#FEE2E2` | `bg-danger-bg` | Danger background |
| `--color-danger-text` | `#991B1B` | `text-danger-text` | Danger text |
| `--color-warning-bg` | `#FEF9C3` | `bg-warning-bg` | Warning background |
| `--color-warning-text` | `#854D0E` | `text-warning-text` | Warning text |
| `--color-info-bg` | `#DBEAFE` | `bg-info-bg` | Info background |
| `--color-info-text` | `#1E40AF` | `text-info-text` | Info text |

### Pathway Card Palettes

| Token | Value | Purpose |
|---|---|---|
| `--color-pathway-warm` | `#F4D9B8` | "I Want to Support" card bg |
| `--color-pathway-warm-border` | `#ECC89E` | Border |
| `--color-pathway-sage` | `#B8CFC4` | "I Need Help" card bg |
| `--color-pathway-sage-border` | `#A0BFAF` | Border |
| `--color-pathway-lavender` | `#C4BAD4` | "Community Events" card bg |
| `--color-pathway-lavender-border` | `#B0A4C4` | Border |

Each pathway also has `-hover` and `-hover-border` variants.

### Radius

| Token | Value | Utility Class |
|---|---|---|
| `--radius-sm` | `0.375rem` | `rounded-sm` |
| `--radius-md` | `0.5rem` | `rounded-md` |
| `--radius-lg` | `0.75rem` | `rounded-lg` |
| `--radius-xl` | `1rem` | `rounded-xl` |
| `--radius-2xl` | `1.25rem` | `rounded-2xl` |
| `--radius-full` | `9999px` | `rounded-full` |

### Shadows

| Token | Value | Utility Class |
|---|---|---|
| `--shadow-sm` | subtle 1px | `shadow-sm` |
| `--shadow-card` | light elevation | `shadow-card` |
| `--shadow-card-hover` | medium elevation | `shadow-card-hover` |
| `--shadow-dialog` | heavy elevation | `shadow-dialog` |

---

## Components

All live in `components/ui/`. Import from the barrel:

```tsx
import { Button, Badge, Card, Input, Textarea, Alert, Dialog } from "@/components/ui";
```

### `cn()` utility — `lib/utils.ts`

Conditional class name joiner. Filters out falsy values.

```tsx
cn("base", isActive && "active", size === "lg" && "text-lg")
// → "base active text-lg" (when both conditions are true)
```

---

### Button

Primary interactive element. Supports rendering as `<button>` or `<Link>`.

**Props:**
| Prop | Type | Default | Description |
|---|---|---|---|
| `variant` | `"primary" \| "secondary" \| "danger" \| "success" \| "admin" \| "ghost"` | `"primary"` | Visual style |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | Size |
| `pill` | `boolean` | `false` | Fully rounded corners |
| `loading` | `boolean` | `false` | Shows spinner, disables button |
| `href` | `string` | — | If set, renders as Next.js `<Link>` |
| `disabled` | `boolean` | — | Standard HTML disabled |

**Usage:**

```tsx
<Button variant="primary" size="md">Submit</Button>
<Button variant="secondary" pill>Cancel</Button>
<Button variant="admin" loading={isPending}>Save</Button>
<Button href="/about" variant="ghost" size="sm">Learn More</Button>
```

**Variants:**
- `primary` — Dark charcoal background, white text (main CTA)
- `secondary` — Transparent with border (secondary action)
- `danger` — Rose/red (destructive actions)
- `success` — Emerald/green (approve actions)
- `admin` — Amber (admin-specific actions)
- `ghost` — No border, subtle hover (minimal UI)

---

### Badge

Status labels and tags.

**Props:**
| Prop | Type | Default | Description |
|---|---|---|---|
| `variant` | `"default" \| "success" \| "warning" \| "danger" \| "info" \| "service" \| "opportunity" \| "business"` | `"default"` | Color scheme |
| `size` | `"sm" \| "md"` | `"sm"` | Size |
| `pill` | `boolean` | `true` | Fully rounded |
| `color` | `string` | — | Dynamic hex color (overrides variant) |

**Usage:**

```tsx
<Badge variant="success">Approved</Badge>
<Badge variant="danger">Urgent</Badge>
<Badge variant="info">Service</Badge>
<Badge color="#8B5CF6">Custom Tag</Badge>
```

The `color` prop enables dynamic tag styling from the API — pass a hex color and it automatically creates a light background (`color + "20"` for 12% opacity) with the color as text.

---

### Card

Container component with border/shadow variants.

**Props:**
| Prop | Type | Default | Description |
|---|---|---|---|
| `variant` | `"default" \| "elevated" \| "interactive"` | `"default"` | Style |
| `padding` | `"none" \| "sm" \| "md" \| "lg"` | `"md"` | Inner padding |

**Usage:**

```tsx
<Card>Default card with border</Card>
<Card variant="elevated">Shadow instead of border</Card>
<Card variant="interactive">Hover effect for clickable cards</Card>
<Card padding="lg">Extra padding</Card>
```

**Variants:**
- `default` — White background, border, rounded corners
- `elevated` — White background, shadow (no border)
- `interactive` — Border + hover shadow transition (for clickable items)

---

### Input / Textarea

Form inputs with consistent styling and error states.

**Props (both):**
| Prop | Type | Default | Description |
|---|---|---|---|
| `error` | `boolean \| string` | — | Shows error ring; if string, shows error message below |

Both accept all standard HTML input/textarea attributes and forward refs.

**Usage:**

```tsx
<Input type="text" placeholder="Enter name" />
<Input type="email" error="Invalid email address" />
<Input type="text" disabled />

<Textarea placeholder="Your message..." rows={4} />
<Textarea error={true} />
```

---

### Alert

Status banners for success, error, warning, and info messages.

**Props:**
| Prop | Type | Default | Description |
|---|---|---|---|
| `variant` | `"success" \| "error" \| "warning" \| "info"` | — | Color scheme |
| `title` | `string` | — | Optional bold title |

**Usage:**

```tsx
<Alert variant="success" title="Submitted!">
  Your resource has been submitted for review.
</Alert>

<Alert variant="error">
  Something went wrong. Please try again.
</Alert>

<Alert variant="info">
  <h2 className="font-semibold mb-2">What happens next?</h2>
  <p>We'll review your submission within 24 hours.</p>
</Alert>
```

---

### Dialog

Modal overlay with backdrop, escape-to-close, and body scroll lock.

**Props:**
| Prop | Type | Default | Description |
|---|---|---|---|
| `isOpen` | `boolean` | — | Controls visibility |
| `onClose` | `() => void` | — | Called on backdrop click or Escape |
| `title` | `string` | — | Header with close button |
| `footer` | `ReactNode` | — | Footer slot (for action buttons) |

**Usage:**

```tsx
<Dialog
  isOpen={showDialog}
  onClose={() => setShowDialog(false)}
  title="Confirm Action"
  footer={
    <>
      <Button variant="ghost" onClick={() => setShowDialog(false)}>Cancel</Button>
      <Button variant="danger" onClick={handleDelete}>Delete</Button>
    </>
  }
>
  <p>Are you sure you want to delete this item?</p>
</Dialog>
```

---

## Conventions

1. **Never use raw hex values.** Always use token-based utility classes (`bg-surface`, `text-text-primary`, `border-border`).

2. **Use components for repeated patterns.** Don't re-invent buttons, cards, or inputs — use the primitives.

3. **Admin pages** use the same components but with `variant="admin"` for buttons and standard Tailwind named colors (e.g. `stone-*`, `amber-*`) where admin-specific styling is needed.

4. **Dynamic colors** from the API should use the `Badge` component's `color` prop, not inline styles.

5. **Token naming** follows a namespace convention:
   - `--color-surface-*` — backgrounds
   - `--color-text-*` — text hierarchy
   - `--color-border-*` — borders
   - `--color-action*` — primary interactive
   - `--color-pathway-*` — home page cards
   - `--radius-*` — border radius
   - `--shadow-*` — box shadows

---

## File Structure

```
packages/web/
  app/globals.css          ← All design tokens (@theme block)
  lib/utils.ts             ← cn() helper
  components/ui/
    index.ts               ← Barrel export
    Button.tsx             ← Button / Link
    Badge.tsx              ← Status labels, tags
    Card.tsx               ← Container
    Input.tsx              ← Input + Textarea
    Alert.tsx              ← Status banners
    Dialog.tsx             ← Modal overlay
```
