import { Sprout } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"

/**
 * Visual marker for any entity whose row was inserted by the dev seed
 * script (data/seed.mjs). Appears on every list row and detail page
 * that renders a dummy post / organization / widget / slotted item.
 *
 * Three forms:
 *   <SeedBadge />                     — pill badge (default)
 *   <SeedBadge size="sm" />           — compact pill for dense rows
 *   <SeedBadge icon />                — icon-only (table cells, tight UI)
 *
 * Consistent warning variant keeps the visual language uniform so
 * editors can scan any screen and spot dummy data.
 */
export function SeedBadge({
  className,
  size = "md",
  icon = false,
  title = "Dummy seed data — will not ship in a real edition",
}: {
  className?: string
  size?: "sm" | "md"
  icon?: boolean
  title?: string
}) {
  if (icon) {
    return (
      <span
        aria-label="Seed data"
        title={title}
        className={cn(
          "inline-flex items-center justify-center rounded-full bg-warning-bg text-warning-text",
          size === "sm" ? "size-4" : "size-5",
          className
        )}
      >
        <Sprout className={size === "sm" ? "size-2.5" : "size-3"} />
      </span>
    )
  }

  return (
    <Badge
      variant="warning"
      title={title}
      className={cn("gap-1 uppercase tracking-wider", className)}
    >
      <Sprout />
      SEED
    </Badge>
  )
}

/**
 * Inline wrapper that only renders the badge when `isSeed` is true.
 * Lets callers write:
 *
 *   <SeedBadgeIf isSeed={post.isSeed} />
 *
 * instead of `{post.isSeed && <SeedBadge />}` everywhere.
 */
export function SeedBadgeIf({
  isSeed,
  ...props
}: {
  isSeed: boolean | null | undefined
} & Parameters<typeof SeedBadge>[0]) {
  if (!isSeed) return null
  return <SeedBadge {...props} />
}
