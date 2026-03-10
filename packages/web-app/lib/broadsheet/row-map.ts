/**
 * Row template mapping: CMS row template slugs → broadsheet layout config.
 *
 * Each backend row template slug maps to a Row variant, cell spans, and how
 * posts distribute across cells. The layout engine in the CMS assigns posts
 * to slots (by slot_index); this mapping tells the renderer how to group those
 * slots into visual cells.
 */

import type { RowVariant, CellSpan } from './types';

export interface RowLayout {
  /** CSS variant class for the Row component */
  variant: RowVariant;
  /** Column spans for each cell in the row */
  cells: CellSpan[];
  /** Max posts per cell (slots grouped into cells) */
  postsPerCell: number[];
}

/**
 * Map a CMS row template slug to a broadsheet RowLayout.
 *
 * Backend row templates (from migration 000174):
 *   hero-with-sidebar      → 1 heavy + 3 light  (4 posts total)
 *   hero-full              → 1 heavy             (1 post)
 *   three-column           → 3 medium            (3 posts, 1 per col)
 *   two-column-wide-narrow → 1 heavy + 1 medium  (2 posts)
 *   classifieds            → 6 light             (6 posts, 2 per col)
 *   ticker                 → up to 8 light       (rendered as standalone strips)
 *   single-medium          → 1 medium            (1 post)
 */
export function getRowLayout(slug: string): RowLayout {
  switch (slug) {
    case 'hero-with-sidebar':
      // Prototype: lead-stack with span=4 hero + span=2 sidebar stacking up to 3 items
      return {
        variant: 'lead-stack',
        cells: [4, 2],
        postsPerCell: [1, 3],
      };

    case 'hero-full':
      return {
        variant: 'full',
        cells: [6],
        postsPerCell: [1],
      };

    case 'three-column':
      // Prototype: trio with exactly 1 post per span=2 column
      return {
        variant: 'trio',
        cells: [2, 2, 2],
        postsPerCell: [1, 1, 1],
      };

    case 'two-column-wide-narrow':
      // Prototype: lead with 1 heavy in span=4 + 1 medium in span=2
      return {
        variant: 'lead',
        cells: [4, 2],
        postsPerCell: [1, 1],
      };

    case 'four-column':
      // Deleted in migration 000176 — kept as fallback
      return {
        variant: 'pair',
        cells: [3, 3],
        postsPerCell: [2, 2],
      };

    case 'classifieds':
      // 6 light posts in 3 columns, 2 stacked per column
      return {
        variant: 'trio',
        cells: [2, 2, 2],
        postsPerCell: [2, 2, 2],
      };

    case 'ticker':
      // Ticker items are full-width standalone strips.
      // postsPerCell is generous so nothing gets dropped.
      return {
        variant: 'full',
        cells: [6],
        postsPerCell: [10],
      };

    case 'single-medium':
      return {
        variant: 'full',
        cells: [6],
        postsPerCell: [1],
      };

    default:
      // Unknown template — fall back to full-width single cell
      return {
        variant: 'full',
        cells: [6],
        postsPerCell: [10],
      };
  }
}

/**
 * Distribute slots into cells based on the row layout.
 *
 * Slots come sorted by slot_index from the API. We greedily fill
 * cells up to their postsPerCell limit. Any overflow slots are
 * appended to the last cell so nothing is silently dropped.
 */
export function distributeSlots<T extends { slotIndex: number }>(
  slots: readonly T[],
  layout: RowLayout
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

  // Overflow: append remaining slots to the last cell
  const lastCell = cells[cells.length - 1];
  while (slotIdx < sorted.length) {
    lastCell.push(sorted[slotIdx]);
    slotIdx++;
  }

  return cells;
}
