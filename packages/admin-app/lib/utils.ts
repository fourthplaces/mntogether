import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * Merges class names with Tailwind-aware deduplication.
 *
 * - `clsx` handles conditional values (booleans, arrays, objects)
 * - `twMerge` resolves conflicting Tailwind utilities (e.g. "px-4" + "px-6" → "px-6")
 *
 * Usage:
 *   cn("base-class", isActive && "active", size === "lg" && "text-lg")
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
