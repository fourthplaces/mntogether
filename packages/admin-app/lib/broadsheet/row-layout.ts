/**
 * Row layout resolver for the admin edition editor.
 *
 * Mirrors `packages/web-app/lib/broadsheet/row-map.ts` so the admin's DnD
 * grid matches how the public broadsheet actually renders each row. Ported
 * here (rather than cross-imported from web-app) because the admin-app and
 * web-app are separate Next.js apps and `@rooteditorial/shared` currently
 * only contains GraphQL code.
 */

export type CellSpan = 1 | 2 | 3 | 4 | 6;
export type RowVariant =
  | "lead"
  | "lead-stack"
  | "pair"
  | "pair-stack"
  | "trio"
  | "trio-mixed"
  | "full";

export interface RowLayout {
  variant: RowVariant;
  /** Column spans for each visual cell in the 6-column grid */
  cells: CellSpan[];
  /** How many template slots belong in each cell */
  postsPerCell: number[];
}

/**
 * Map a layoutVariant + slotCount to a visual RowLayout.
 *
 * layoutVariant values:
 *   full       → 1 cell spanning all 6 columns
 *   lead       → [4, 2] — one wide lead + one narrow companion
 *   lead-stack → [4, 2] — one wide lead + stacked posts in the narrow cell
 *   pair       → [3, 3] — two equal half-columns
 *   pair-stack → [3, 3] — one lead + stacked posts in the second cell
 *   trio       → [2, 2, 2] — three equal columns
 */
export function getRowLayout(layoutVariant: string, slotCount?: number): RowLayout {
  switch (layoutVariant) {
    case "lead-stack":
      return {
        variant: "lead-stack",
        cells: [4, 2],
        postsPerCell: [1, Math.max((slotCount ?? 4) - 1, 1)],
      };

    case "trio":
      if (slotCount && slotCount > 3) {
        const perCol = Math.ceil(slotCount / 3);
        return {
          variant: "trio",
          cells: [2, 2, 2],
          postsPerCell: [perCol, perCol, perCol],
        };
      }
      return {
        variant: "trio",
        cells: [2, 2, 2],
        postsPerCell: [1, 1, 1],
      };

    case "lead":
      return {
        variant: "lead",
        cells: [4, 2],
        postsPerCell: [1, 1],
      };

    case "pair": {
      const pairHalf = Math.ceil((slotCount ?? 2) / 2);
      return {
        variant: "pair",
        cells: [3, 3],
        postsPerCell: [pairHalf, pairHalf],
      };
    }

    case "pair-stack":
      return {
        variant: "pair-stack",
        cells: [3, 3],
        postsPerCell: [1, Math.max((slotCount ?? 5) - 1, 1)],
      };

    case "full":
    default:
      return {
        variant: "full",
        cells: [6],
        postsPerCell: [Math.max(slotCount ?? 1, 1)],
      };
  }
}

/**
 * Distribute template slots (sorted by slotIndex) into visual cells based on
 * `postsPerCell`. Any overflow slots are appended to the last cell so nothing
 * is silently dropped.
 */
export function distributeSlots<T extends { slotIndex: number }>(
  slots: readonly T[],
  layout: RowLayout,
): T[][] {
  const sorted = [...slots].sort((a, b) => a.slotIndex - b.slotIndex);
  const cells: T[][] = layout.cells.map(() => []);
  let slotIdx = 0;

  for (let cellIdx = 0; cellIdx < layout.cells.length; cellIdx++) {
    const max = layout.postsPerCell[cellIdx];
    for (let i = 0; i < max && slotIdx < sorted.length; i++) {
      cells[cellIdx].push(sorted[slotIdx]);
      slotIdx++;
    }
  }

  const lastCell = cells[cells.length - 1];
  while (slotIdx < sorted.length) {
    lastCell.push(sorted[slotIdx]);
    slotIdx++;
  }

  return cells;
}

/**
 * Tailwind `col-span-{n}` helper, mapped out explicitly so the JIT keeps them.
 * (Dynamic class names like `col-span-${n}` get tree-shaken by the scanner.)
 */
export function cellSpanClass(span: CellSpan): string {
  switch (span) {
    case 1: return "col-span-1";
    case 2: return "col-span-2";
    case 3: return "col-span-3";
    case 4: return "col-span-4";
    case 6: return "col-span-6";
  }
}
