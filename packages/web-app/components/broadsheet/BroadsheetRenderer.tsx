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
import { SectionSep, ResourceBar, StatCard, NumberBlock, PullQuote } from '@/components/broadsheet';
import { getRowLayout, distributeSlots } from '@/lib/broadsheet/row-map';
import { resolveTemplate } from '@/lib/broadsheet/templates';
import { preparePost } from '@/lib/broadsheet/prepare';

type BroadsheetData = NonNullable<PublicBroadsheetQuery['publicBroadsheet']>;
type BroadsheetRowData = BroadsheetData['rows'][number];
type BroadsheetSlotData = BroadsheetRowData['slots'][number];

interface BroadsheetRendererProps {
  edition: BroadsheetData;
}

export function BroadsheetRenderer({ edition }: BroadsheetRendererProps) {
  const rows = [...edition.rows].sort((a, b) => a.sortOrder - b.sortOrder);

  // Group rows by section
  const ungroupedRows = rows.filter((r) => !r.sectionId);
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

      {/* Above the fold — rows without a section */}
      {ungroupedRows.map((row, idx) => (
        <BroadsheetRow key={`ungrouped-row-${idx}`} row={row} />
      ))}

      {/* Topic sections — rows interleaved by sort_order */}
      {sortedSections.map((section) => {
        const sectionRows = rows.filter((r) => r.sectionId === section.id);
        if (sectionRows.length === 0) return null;
        return (
          <div key={section.id} data-section={section.topicSlug ?? section.id}>
            {sectionRows.map((row, idx) => (
              <BroadsheetRow key={`section-row-${idx}`} row={row} />
            ))}
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
  // Widget-standalone rows render the widget directly without Row/Cell wrapper
  if (row.layoutVariant === 'widget-standalone') {
    const widgetSlot = row.slots.find((s) => s.kind === 'widget');
    if (!widgetSlot?.widget) return null;
    return <WidgetRenderer widget={widgetSlot.widget} widgetTemplate={widgetSlot.widgetTemplate ?? undefined} />;
  }

  // Filter to only post slots for the row layout engine
  const postSlots = row.slots.filter((s) => s.kind === 'post' && s.post);
  const layout = getRowLayout(row.layoutVariant ?? 'full', postSlots.length);

  // Distribute post slots into cells
  const cellSlots = distributeSlots(postSlots, layout);

  return (
    <Row variant={layout.variant}>
      {cellSlots.map((slots, cellIdx) => (
        <Cell key={cellIdx} span={layout.cells[cellIdx]}>
          {slots.map((slot) => (
            <SlotRenderer key={slot.post!.id} slot={slot} />
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
  if (slot.kind === 'widget' && slot.widget) {
    return <WidgetRenderer widget={slot.widget} widgetTemplate={slot.widgetTemplate ?? undefined} />;
  }

  if (!slot.post || !slot.postTemplate) return null;

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
// Widget renderer — reads widget data and maps widgetType → component
// =============================================================================

type WidgetData = NonNullable<BroadsheetSlotData['widget']>;

function WidgetRenderer({ widget, widgetTemplate }: { widget: WidgetData; widgetTemplate?: string }) {
  let data: Record<string, unknown> = {};
  try {
    data = typeof widget.data === 'string'
      ? JSON.parse(widget.data)
      : (widget.data as Record<string, unknown>) ?? {};
  } catch {
    return null;
  }

  switch (widget.widgetType) {
    case 'section_header':
    case 'section_sep':
      return (
        <SectionSep
          title={(data.title as string) || 'Section'}
          sub={data.sub as string | undefined}
          variant={widgetTemplate === 'ledger' ? 'ledger' : 'default'}
        />
      );

    case 'number':
    case 'stat_card':
    case 'number_block': {
      // Merged "number" type — variant selects visual treatment.
      // stat_card/number_block kept as aliases for backward compat.
      const variant = widgetTemplate || widget.widgetType;
      if (variant === 'number_block' || variant === 'number-block') {
        return (
          <NumberBlock
            number={(data.number as string) || ''}
            label={(data.label as string) || (data.title as string) || ''}
            detail={data.detail as string | undefined}
            color={(data.color as string) || 'teal'}
          />
        );
      }
      // Default: stat-card rendering
      return (
        <StatCard
          number={(data.number as string) || ''}
          title={(data.title as string) || (data.label as string) || ''}
          body={(data.body as string) || (data.detail as string) || ''}
        />
      );
    }

    case 'pull_quote':
      return (
        <PullQuote
          quote={(data.quote as string) || ''}
          attribution={(data.attribution as string) || ''}
        />
      );

    case 'resource_bar':
      return (
        <ResourceBar
          label={(data.label as string) || 'Resources'}
          items={
            Array.isArray(data.items)
              ? (data.items as Array<{ number: string; text: string }>)
              : []
          }
        />
      );

    case 'weather':
      return (
        <div className="widget-placeholder" data-widget={widget.widgetType}>
          <span className="mono-sm">Weather widget (data pending)</span>
        </div>
      );

    default:
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
