'use client';

/**
 * BroadsheetRenderer — takes the PublicBroadsheet query result and renders
 * the full newspaper layout: BroadsheetHeader → Masthead → Rows + Widgets.
 *
 * Rows and widgets are independent layout items, interleaved by sort_order.
 * Each row maps a CMS row template slug to a Row variant (lead, trio, full, etc.),
 * distributes its slots into cells, and resolves each slot to the correct post component.
 * Widgets (section headers, weather, resource bars) are rendered between rows.
 */

import type { PublicBroadsheetQuery } from '@/gql/graphql';
import { Row, Cell, NewspaperFrame, DebugLabels } from '@/components/broadsheet';
import { SectionSep, ResourceBar } from '@/components/broadsheet';
import { getRowLayout, distributeSlots } from '@/lib/broadsheet/row-map';
import { resolveTemplate } from '@/lib/broadsheet/templates';
import { preparePost } from '@/lib/broadsheet/prepare';

type BroadsheetData = NonNullable<PublicBroadsheetQuery['publicBroadsheet']>;
type BroadsheetRowData = BroadsheetData['rows'][number];
type BroadsheetSlotData = BroadsheetRowData['slots'][number];
type BroadsheetWidgetData = BroadsheetData['widgets'][number];
type BroadsheetSectionData = BroadsheetData['sections'][number];

type LayoutItem =
  | { type: 'row'; data: BroadsheetRowData; sortOrder: number; sectionId?: string | null }
  | { type: 'widget'; data: BroadsheetWidgetData; sortOrder: number; sectionId?: string | null };

interface BroadsheetRendererProps {
  edition: BroadsheetData;
}

export function BroadsheetRenderer({ edition }: BroadsheetRendererProps) {
  // Build unified layout items from rows and widgets
  const allItems: LayoutItem[] = [
    ...edition.rows.map((r) => ({
      type: 'row' as const,
      data: r,
      sortOrder: r.sortOrder,
      sectionId: r.sectionId,
    })),
    ...(edition.widgets ?? []).map((w) => ({
      type: 'widget' as const,
      data: w,
      sortOrder: w.sortOrder,
      sectionId: w.sectionId,
    })),
  ].sort((a, b) => a.sortOrder - b.sortOrder);

  // Group items by section
  const ungroupedItems = allItems.filter((item) => !item.sectionId);
  const sortedSections = [...edition.sections].sort((a, b) => a.sortOrder - b.sortOrder);

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

      {/* Above the fold — items without a section */}
      {ungroupedItems.map((item, idx) =>
        item.type === 'row' ? (
          <BroadsheetRow key={`ungrouped-row-${idx}`} row={item.data} />
        ) : (
          <WidgetRenderer key={`ungrouped-widget-${idx}`} widget={item.data} />
        )
      )}

      {/* Topic sections — items interleaved by sort_order */}
      {sortedSections.map((section) => {
        const sectionItems = allItems.filter((item) => item.sectionId === section.id);
        if (sectionItems.length === 0) return null;
        return (
          <div key={section.id} data-section={section.topicSlug ?? section.id}>
            {sectionItems.map((item, idx) =>
              item.type === 'row' ? (
                <BroadsheetRow key={`section-row-${idx}`} row={item.data} />
              ) : (
                <WidgetRenderer key={`section-widget-${idx}`} widget={item.data} />
              )
            )}
          </div>
        );
      })}
    </NewspaperFrame>
  );
}

// =============================================================================
// Row renderer
// =============================================================================

function BroadsheetRow({ row }: { row: BroadsheetRowData }) {
  const layout = getRowLayout(row.layoutVariant ?? 'full', row.slots.length);

  // Distribute slots into cells
  const cellSlots = distributeSlots(row.slots, layout);

  return (
    <Row variant={layout.variant}>
      {cellSlots.map((slots, cellIdx) => (
        <Cell key={cellIdx} span={layout.cells[cellIdx]}>
          {slots.map((slot) => (
            <SlotRenderer key={slot.post.id} slot={slot} />
          ))}
        </Cell>
      ))}
    </Row>
  );
}

// =============================================================================
// Slot renderer — resolves template + type → component, prepares post data
// =============================================================================

function SlotRenderer({ slot }: { slot: BroadsheetSlotData }) {
  const Component = resolveTemplate(slot.postTemplate, slot.post.postType);
  const post = preparePost(slot.post, slot.postTemplate);

  return (
    <a
      href={`/posts/${slot.post.id}`}
      className="post-link"
      style={{ textDecoration: 'none', color: 'inherit', display: 'block' }}
    >
      <Component data={post} />
    </a>
  );
}

// =============================================================================
// Widget renderer — parses config JSON and maps widgetType → component
// =============================================================================

function WidgetRenderer({ widget }: { widget: BroadsheetWidgetData }) {
  let config: Record<string, unknown> = {};
  try {
    config = typeof widget.config === 'string'
      ? JSON.parse(widget.config)
      : (widget.config as Record<string, unknown>);
  } catch {
    // Invalid JSON — render nothing
    return null;
  }

  switch (widget.widgetType) {
    case 'section_header':
      return (
        <SectionSep
          title={(config.title as string) || 'Section'}
          sub={config.subtitle as string | undefined}
        />
      );

    case 'section_sep':
      return (
        <SectionSep
          title={(config.title as string) || ''}
          sub={config.subtitle as string | undefined}
        />
      );

    case 'hotline_bar':
      return (
        <ResourceBar
          label={(config.label as string) || 'Resources'}
          items={
            Array.isArray(config.items)
              ? (config.items as Array<{ number: string; text: string }>)
              : []
          }
        />
      );

    // Weather widgets need real data — render placeholder for now
    case 'weather':
      return (
        <div className="widget-placeholder" data-widget={widget.widgetType}>
          <span className="mono-sm">Weather widget (data pending)</span>
        </div>
      );

    default:
      // Unknown widget type — skip
      return null;
  }
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
