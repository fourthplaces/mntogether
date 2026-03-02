/**
 * Edition staleness utilities.
 *
 * Computes how "stale" an edition is based on its periodEnd date relative to
 * today. Used across the kanban, dashboard, and edition detail pages to show
 * human-readable freshness labels with escalating visual severity.
 */

// ─── Types ───────────────────────────────────────────────────────────────────

export type StalenessLevel = "current" | "recent" | "warning" | "alert";

// ─── Calculation ─────────────────────────────────────────────────────────────

/** How many full weeks have elapsed since the edition's period ended. */
export function getWeeksOld(periodEnd: string): number {
  const end = new Date(periodEnd + "T23:59:59");
  const now = new Date();
  const diffMs = now.getTime() - end.getTime();
  return Math.max(0, Math.floor(diffMs / (7 * 24 * 60 * 60 * 1000)));
}

/** Map weeks-old to a severity level for styling. */
export function getStalenessLevel(weeksOld: number): StalenessLevel {
  if (weeksOld === 0) return "current";
  if (weeksOld === 1) return "recent";
  if (weeksOld === 2) return "warning";
  return "alert";
}

/** Human-readable staleness label. */
export function getStalenessLabel(weeksOld: number): string {
  if (weeksOld === 0) return "Current";
  if (weeksOld === 1) return "1 week old";
  return `${weeksOld} weeks old`;
}

// ─── Tailwind class maps ─────────────────────────────────────────────────────

export const STALENESS_BORDER: Record<StalenessLevel, string> = {
  current: "border-l-stone-300",
  recent:  "border-l-stone-300",
  warning: "border-l-amber-400",
  alert:   "border-l-red-400",
};

export const STALENESS_TEXT: Record<StalenessLevel, string> = {
  current: "text-stone-500",
  recent:  "text-stone-500",
  warning: "text-amber-600",
  alert:   "text-red-600",
};

export const STALENESS_BG: Record<StalenessLevel, string> = {
  current: "",
  recent:  "",
  warning: "bg-amber-50",
  alert:   "bg-red-50",
};
