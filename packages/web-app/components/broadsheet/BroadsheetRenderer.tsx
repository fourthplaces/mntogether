'use client';

/**
 * BroadsheetRenderer — takes the PublicBroadsheet query result and renders
 * the full newspaper layout: BroadsheetHeader → Masthead → Rows.
 *
 * Rows contain polymorphic slots (post or widget). Each row maps a CMS row
 * template slug to a Row variant (lead, trio, full, etc.), distributes its
 * slots into cells, and resolves each slot to the correct post or widget
 * component. Standalone widget rows (widget-standalone) render their widget
 * directly without a Row/Cell wrapper.
 */

import type { PublicBroadsheetQuery } from '@/gql/graphql';
import type { MouseEvent as ReactMouseEvent, ReactNode } from 'react';
import { useRouter } from 'next/navigation';
import { Row, Cell, NewspaperFrame, DebugLabels } from '@/components/broadsheet';
import { getRowLayout, distributeSlots } from '@/lib/broadsheet/row-map';
import { resolveTemplate } from '@/lib/broadsheet/templates';
import { resolveWidget } from '@/lib/broadsheet/widget-resolver';
import { preparePost, type PostTemplateConfigMap } from '@/lib/broadsheet/prepare';
import { PostDetailLinkProvider } from '@/lib/broadsheet/post-link-context';

type BroadsheetData = NonNullable<PublicBroadsheetQuery['publicBroadsheet']>;
type BroadsheetRowData = BroadsheetData['rows'][number];
type BroadsheetSlotData = BroadsheetRowData['slots'][number];

interface BroadsheetRendererProps {
  edition: BroadsheetData;
  /**
   * Post template configs (body_target, body_max) keyed by slug. Fetched
   * via PostTemplateConfigsQuery at the page level and threaded in so
   * preparePost can enforce body limits from the DB rather than a
   * hardcoded duplicate. When omitted, a conservative fallback is used.
   */
  templateConfigs?: PostTemplateConfigMap;
  /**
   * When true, post cards (and their "Read more" CTAs) link to the
   * admin-only /preview/posts/[id] route instead of the public
   * /posts/[id]. Set by /preview/[editionId] so editors walking a draft
   * edition can click through to each post's full-detail preview.
   */
  previewMode?: boolean;
}

export function BroadsheetRenderer({ edition, templateConfigs, previewMode }: BroadsheetRendererProps) {
  // Render ALL rows in sort_order — flat, no section grouping.
  // Sections are kept as advisory metadata for the admin editor but
  // don't affect public rendering. Visual breaks come from SectionSep
  // widgets placed explicitly by the layout engine.
  const rows = [...edition.rows].sort((a, b) => a.sortOrder - b.sortOrder);

  return (
    <NewspaperFrame>
      <DebugLabels />

      {/* Broadsheet header — sits above the masthead */}
      <div className="broadsheet-header">
        <a href="/about">Strength in Community</a>
        <a href="/contact">Contact MNTogether.org</a>
      </div>

      {/* Masthead — dynamic edition data */}
      <header className="masthead">
        <h1>Minnesota, Together.</h1>
        <div className="dateline">
          Printed:{' '}
          <span className="handwritten">
            {formatDate(edition.periodStart)}
          </span>
          {' '}&mdash; Edition:{' '}
          <span className="handwritten">{edition.county.name}</span>
        </div>
      </header>

      {/* All rows in sort_order — widgets and posts interleaved */}
      {rows.map((row, idx) => (
        <BroadsheetRow
          key={`row-${idx}`}
          row={row}
          templateConfigs={templateConfigs}
          previewMode={previewMode}
        />
      ))}
    </NewspaperFrame>
  );
}

// =============================================================================
// Row renderer
// =============================================================================

function BroadsheetRow({
  row,
  templateConfigs,
  previewMode,
}: {
  row: BroadsheetRowData;
  templateConfigs?: PostTemplateConfigMap;
  previewMode?: boolean;
}) {
  // Widget-standalone rows render the widget directly without Row/Cell wrapper
  if (row.layoutVariant === 'widget-standalone') {
    const widgetSlot = row.slots.find((s) => s.kind === 'widget');
    if (!widgetSlot?.widget) return null;
    return <WidgetRenderer widget={widgetSlot.widget} widgetTemplate={widgetSlot.widgetTemplate ?? undefined} />;
  }

  // Multi-widget rows (trio/pair of widgets): render each widget in a cell
  const widgetSlots = row.slots.filter((s) => s.kind === 'widget' && s.widget);
  if (widgetSlots.length > 0 && row.slots.every((s) => s.kind === 'widget')) {
    const layout = getRowLayout(row.layoutVariant ?? 'trio', widgetSlots.length);
    return (
      <Row variant={layout.variant}>
        {widgetSlots.map((slot, idx) => (
          <Cell key={`widget-${idx}`} span={layout.cells[idx] ?? layout.cells[layout.cells.length - 1]}>
            <WidgetRenderer widget={slot.widget!} widgetTemplate={slot.widgetTemplate ?? undefined} />
          </Cell>
        ))}
      </Row>
    );
  }

  // Filter to only post slots for the row layout engine
  const postSlots = row.slots.filter((s) => s.kind === 'post' && s.post);
  if (postSlots.length === 0) return null;

  const layout = getRowLayout(row.layoutVariant ?? 'full', postSlots.length);

  // Distribute post slots into cells
  const cellSlots = distributeSlots(postSlots, layout);

  // Skip rows where any cell is empty — indicates an unfilled layout
  const hasEmptyCell = cellSlots.some((slots) => slots.length === 0);
  if (hasEmptyCell) return null;

  return (
    <Row variant={layout.variant}>
      {cellSlots.map((slots, cellIdx) => (
        <Cell key={cellIdx} span={layout.cells[cellIdx]}>
          {slots.map((slot) => (
            <SlotRenderer
              key={slot.post!.id}
              slot={slot}
              isAnchor={cellIdx === 0 && slots.length === 1}
              templateConfigs={templateConfigs}
              previewMode={previewMode}
            />
          ))}
        </Cell>
      ))}
    </Row>
  );
}

// =============================================================================
// Slot renderer — resolves template + type → component, prepares post data
// =============================================================================

function SlotRenderer({
  slot,
  isAnchor,
  templateConfigs,
  previewMode,
}: {
  slot: BroadsheetSlotData;
  isAnchor?: boolean;
  templateConfigs?: PostTemplateConfigMap;
  previewMode?: boolean;
}) {
  if (slot.kind === 'widget' && slot.widget) {
    return <WidgetRenderer widget={slot.widget} widgetTemplate={slot.widgetTemplate ?? undefined} />;
  }

  if (!slot.post || !slot.postTemplate) return null;

  const Component = resolveTemplate(slot.postTemplate, slot.post.postType);
  const post = preparePost(slot.post, slot.postTemplate, isAnchor, templateConfigs);

  // The card's title is the clickable affordance for the detail page.
  // In preview mode it links to /preview/posts/[id] so editors walking
  // a draft edition can click through to each post's full preview;
  // otherwise, the public /posts/[id] route. The href travels via
  // React context so MTitle can read it without every post card
  // component having to plumb a new prop.
  const detailHref = previewMode
    ? `/preview/posts/${slot.post.id}`
    : `/posts/${slot.post.id}`;

  return (
    <PostDetailLinkProvider value={detailHref}>
      <ClickableTile href={detailHref}>
        <Component data={post} />
      </ClickableTile>
    </PostDetailLinkProvider>
  );
}

// =============================================================================
// ClickableTile — whole-tile click affordance
// =============================================================================
//
// Makes the entire post card clickable without breaking:
//   - native links/buttons inside the card (title anchor, Read more, contacts)
//   - text selection (click-and-drag to select body text)
//   - CSS :hover states (background tint, title underline)
//
// Approach: plain <div> onClick handler that bails out if
//   (a) the click originated on a real interactive element — let that
//       element's native behavior win; we don't double-navigate.
//   (b) the user has text selected — they were dragging, not clicking.
// Otherwise, programmatic router.push() to the detail page.
//
// Why not an absolute-positioned <a> overlay (the previous approach)?
// Overlays capture pointer events, which kills native text selection and
// defeats nested link hover states via CSS specificity. onClick bubbles
// naturally — selection and inner links "just work".
//
// Why not <Link> wrapping? HTML forbids <a> inside <a>, and the card's
// title is already rendered as an <a> via MTitle + PostDetailLinkProvider.
// Nesting would produce invalid markup.

const INTERACTIVE_SELECTOR = 'a, button, [role="button"], input, textarea, select, label';

function ClickableTile({ href, children }: { href: string; children: ReactNode }) {
  const router = useRouter();

  const onClick = (e: ReactMouseEvent<HTMLDivElement>) => {
    // Let native interactive elements handle their own click.
    const target = e.target as HTMLElement | null;
    if (target?.closest(INTERACTIVE_SELECTOR)) return;

    // If the user selected any text during this click, treat it as a drag
    // (text selection), not a navigation intent.
    if (typeof window !== 'undefined' && window.getSelection()?.toString()) return;

    // Only handle plain left-clicks. Let cmd/ctrl/shift/middle-click be
    // picked up by the title's <a> via `target?.closest`… but since we're
    // here the target wasn't interactive, so those modifiers just do
    // nothing (no "open in new tab" from a non-link). Acceptable.
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;

    router.push(href);
  };

  return (
    <div className="post-tile-clickable" onClick={onClick}>
      {children}
    </div>
  );
}

// =============================================================================
// Widget renderer — reads widget data and maps widgetType → component
// =============================================================================

type WidgetData = NonNullable<BroadsheetSlotData['widget']>;

function WidgetRenderer({ widget, widgetTemplate }: { widget: WidgetData; widgetTemplate?: string }) {
  return resolveWidget(
    { widgetType: widget.widgetType, data: widget.data },
    widgetTemplate
  );
}

// =============================================================================
// Helpers
// =============================================================================

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr + 'T00:00:00');
    return d.toLocaleDateString('en-US', {
      month: 'long',
      day: 'numeric',
      year: 'numeric',
    });
  } catch {
    return dateStr;
  }
}
