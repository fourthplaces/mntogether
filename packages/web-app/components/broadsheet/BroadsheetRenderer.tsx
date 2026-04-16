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
import { Row, Cell, NewspaperFrame, DebugLabels } from '@/components/broadsheet';
import { getRowLayout, distributeSlots } from '@/lib/broadsheet/row-map';
import { resolveTemplate } from '@/lib/broadsheet/templates';
import { resolveWidget } from '@/lib/broadsheet/widget-resolver';
import { preparePost, type PostTemplateConfigMap } from '@/lib/broadsheet/prepare';

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
}

export function BroadsheetRenderer({ edition, templateConfigs }: BroadsheetRendererProps) {
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
        <BroadsheetRow key={`row-${idx}`} row={row} templateConfigs={templateConfigs} />
      ))}
    </NewspaperFrame>
  );
}

// =============================================================================
// Row renderer
// =============================================================================

function BroadsheetRow({ row, templateConfigs }: { row: BroadsheetRowData; templateConfigs?: PostTemplateConfigMap }) {
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
}: {
  slot: BroadsheetSlotData;
  isAnchor?: boolean;
  templateConfigs?: PostTemplateConfigMap;
}) {
  if (slot.kind === 'widget' && slot.widget) {
    return <WidgetRenderer widget={slot.widget} widgetTemplate={slot.widgetTemplate ?? undefined} />;
  }

  if (!slot.post || !slot.postTemplate) return null;

  const Component = resolveTemplate(slot.postTemplate, slot.post.postType);
  const post = preparePost(slot.post, slot.postTemplate, isAnchor, templateConfigs);

  // "Linked card" pattern: the whole card is clickable to the post detail page,
  // but inner interactive elements (CTA buttons, external links in references)
  // keep their own behavior. A spanning overlay <a> creates the card-click
  // target; CSS lifts inner <a>/<button> above it so they take precedence.
  // This avoids nested-anchor HTML which is invalid and breaks hydration.
  return (
    <div className="post-link">
      <a
        href={`/posts/${slot.post.id}`}
        className="post-link__overlay"
        aria-label={`Read: ${slot.post.title}`}
      />
      <Component data={post} />
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
