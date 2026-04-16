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
 * Map a layout variant + slot count to a broadsheet RowLayout.
 *
 * Layout variants are CSS grid configurations shared by many row template
 * recipes. For example, "hero-feature-digest" and "hero-feature-ticker" both
 * use the `lead-stack` variant — they differ only in which post templates
 * the slots use, not the grid.
 *
 * Layout variants:
 *   lead-stack → 4+2 grid: span-4 hero + span-2 sidebar stacking items
 *   trio       → 2+2+2 grid: three equal columns
 *   full       → 6 grid: full-width single column
 *   lead       → 4+2 grid: wide lead + narrow companion
 *   pair       → 3+3 grid: two equal columns
 *   pair-stack → 3+3 grid: single lead + stacked posts in second cell
 */
export function getRowLayout(layoutVariant: string, slotCount?: number): RowLayout {
  switch (layoutVariant) {
    case 'lead-stack':
      return {
        variant: 'lead-stack',
        cells: [4, 2],
        postsPerCell: [1, Math.max((slotCount ?? 4) - 1, 1)],
      };

    case 'trio':
      // Distribute slots evenly across 3 columns
      if (slotCount && slotCount > 3) {
        const perCol = Math.ceil(slotCount / 3);
        return {
          variant: 'trio',
          cells: [2, 2, 2],
          postsPerCell: [perCol, perCol, perCol],
        };
      }
      return {
        variant: 'trio',
        cells: [2, 2, 2],
        postsPerCell: [1, 1, 1],
      };

    case 'lead':
      return {
        variant: 'lead',
        cells: [4, 2],
        postsPerCell: [1, 1],
      };

    case 'pair': {
      // Pair rows vary from 2 slots (pair-bulletin-ledger, pair-digest) to
      // 4 slots (pair-bulletin-event). Distribute evenly across 2 cells.
      const pairHalf = Math.ceil((slotCount ?? 2) / 2);
      return {
        variant: 'pair',
        cells: [3, 3],
        postsPerCell: [pairHalf, pairHalf],
      };
    }

    case 'pair-stack':
      return {
        variant: 'pair-stack',
        cells: [3, 3],
        postsPerCell: [1, Math.max((slotCount ?? 5) - 1, 1)],
      };

    case 'full':
    default:
      return {
        variant: 'full',
        cells: [6],
        postsPerCell: [Math.max(slotCount ?? 1, 1)],
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
