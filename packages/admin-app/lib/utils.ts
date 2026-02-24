/**
 * Conditional class name joiner.
 * Filters out falsy values and joins the rest with spaces.
 *
 * Usage:
 *   cn("base-class", isActive && "active", size === "lg" && "text-lg")
 */
export function cn(
  ...classes: (string | boolean | undefined | null)[]
): string {
  return classes.filter(Boolean).join(" ");
}
