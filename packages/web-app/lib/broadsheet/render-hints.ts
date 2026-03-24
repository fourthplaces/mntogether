/**
 * Render hints: computed display fields derived at render time from stored
 * field group data. Pure function, no side effects, trivially testable.
 *
 * These fields are never stored in the CMS — they exist only in the
 * broadsheet rendering pipeline. See POST-DATA-MODEL.md § Renderer Hint Fields.
 */

import type { Post } from './types';
import {
  extractEventParts,
  formatEventLabel,
  formatEventWhen,
  formatPostDate,
} from './dates';

export interface RenderHints {
  // Datetime-derived (events)
  month?: string;
  day?: string;
  dow?: string;
  when?: string;
  circleLabel?: string;

  // Items-derived
  count?: string;

  // Person-derived (spotlights)
  tagline?: string;

  // Body-derived
  pullQuote?: string;

  // Date display (notices, meta)
  date?: string;
}

/**
 * Compute display hint fields from post data + field groups.
 */
export function computeRenderHints(post: Post): RenderHints {
  const hints: RenderHints = {};

  // Datetime hints — for event components (MN timezone)
  if (post.datetime?.start) {
    const parts = extractEventParts(post.datetime.start);
    if (parts) {
      hints.month = parts.month;
      hints.day = parts.day;
      hints.dow = parts.dow;
      hints.circleLabel = formatEventLabel(post.datetime.start);
      hints.when = formatEventWhen(
        post.datetime.start,
        post.datetime.end,
        post.datetime.cost,
        post.datetime.recurring,
      );
    }
  }

  // Items count
  if (post.items?.length) {
    hints.count = post.items.length.toString();
  }

  // Person tagline — role or business description
  if (post.person?.role) {
    hints.tagline = post.person.role;
  }

  // Pull quote — from person quote if available
  if (post.person?.quote) {
    hints.pullQuote = post.person.quote;
  }

  // Date display — from meta timestamp or published date (MN timezone)
  if (post.meta?.timestamp) {
    hints.date = formatPostDate(post.meta.timestamp);
  }

  return hints;
}
