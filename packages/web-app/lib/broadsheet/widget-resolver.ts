/**
 * Widget resolver: centralizes widget rendering logic.
 *
 * Parallel to resolveTemplate() for posts. Owns JSON parsing, variant
 * resolution, and component selection — single entry point for all
 * widget rendering in the broadsheet.
 *
 * See TODO.md #10: "Centralize Widget Rendering (resolveWidget)"
 */

import type { ReactElement } from 'react';
import {
  SectionSep,
  ResourceBar,
  StatCard,
  NumberBlock,
  PullQuote,
} from '@/components/broadsheet';

interface WidgetInput {
  widgetType: string;
  data: string | Record<string, unknown>;
}

/**
 * Parse widget data JSON and resolve to the correct React element.
 *
 * @param widget - The widget record (widgetType + data)
 * @param widgetTemplate - Optional visual variant from the edition slot
 * @returns React element or null if type is unknown / data is invalid
 */
export function resolveWidget(
  widget: WidgetInput,
  widgetTemplate?: string
): ReactElement | null {
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
      return SectionSep({
        title: (data.title as string) || 'Section',
        sub: data.sub as string | undefined,
        variant: widgetTemplate === 'ledger' ? 'ledger' : 'default',
      });

    case 'number':
    case 'stat_card':
    case 'number_block': {
      // Merged "number" type — widget_template selects visual treatment.
      // stat_card/number_block kept as aliases for backward compat.
      const variant = widgetTemplate || widget.widgetType;
      if (variant === 'number_block' || variant === 'number-block') {
        return NumberBlock({
          number: (data.number as string) || '',
          label: (data.label as string) || (data.title as string) || '',
          detail: data.detail as string | undefined,
          color: (data.color as string) || 'teal',
        });
      }
      // Default: stat-card rendering
      return StatCard({
        number: (data.number as string) || '',
        title: (data.title as string) || (data.label as string) || '',
        body: (data.body as string) || (data.detail as string) || '',
      });
    }

    case 'pull_quote':
      return PullQuote({
        quote: (data.quote as string) || '',
        attribution: (data.attribution as string) || '',
      });

    case 'resource_bar':
      return ResourceBar({
        label: (data.label as string) || 'Resources',
        items: Array.isArray(data.items)
          ? (data.items as Array<{ number: string; text: string }>)
          : [],
      });

    case 'weather':
      // Weather widgets require live data — placeholder until API is wired
      return null;

    default:
      return null;
  }
}
