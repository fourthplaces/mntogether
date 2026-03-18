/**
 * Render hints: computed display fields derived at render time from stored
 * field group data. Pure function, no side effects, trivially testable.
 *
 * These fields are never stored in the CMS — they exist only in the
 * broadsheet rendering pipeline. See POST-DATA-MODEL.md § Renderer Hint Fields.
 */

import type { Post } from './types';

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

  // Datetime hints — for event components
  if (post.datetime?.start) {
    const d = new Date(post.datetime.start);
    if (!isNaN(d.getTime())) {
      hints.month = d.toLocaleDateString('en-US', { month: 'short' }).toUpperCase();
      hints.day = d.getDate().toString();
      hints.dow = d.toLocaleDateString('en-US', { weekday: 'short' });
      hints.circleLabel = computeCircleLabel(d);
      hints.when = formatWhen(post);
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

  // Date display — from meta timestamp or published date
  if (post.meta?.timestamp) {
    const d = new Date(post.meta.timestamp);
    if (!isNaN(d.getTime())) {
      hints.date = d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    }
  }

  return hints;
}

// =============================================================================
// Helpers
// =============================================================================

/**
 * Relative date label for the event circle badge.
 * "Today!", "Tomorrow", "Sat", or "Mar 15"
 */
function computeCircleLabel(eventDate: Date): string {
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const target = new Date(eventDate.getFullYear(), eventDate.getMonth(), eventDate.getDate());
  const diffDays = Math.round((target.getTime() - today.getTime()) / (1000 * 60 * 60 * 24));

  if (diffDays === 0) return 'Today!';
  if (diffDays === 1) return 'Tomorrow';
  if (diffDays > 1 && diffDays <= 6) {
    return eventDate.toLocaleDateString('en-US', { weekday: 'short' });
  }
  return eventDate.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

/**
 * Formatted schedule string for event components.
 * "Sat 10am–2pm", "Sat 10am–2pm · Free", "Recurring · Mon 6–7:30pm"
 */
function formatWhen(post: Post): string {
  if (!post.datetime?.start) return '';

  const start = new Date(post.datetime.start);
  if (isNaN(start.getTime())) return '';

  const dow = start.toLocaleDateString('en-US', { weekday: 'short' });
  const startTime = formatTime(start);

  let when = `${dow} ${startTime}`;

  if (post.datetime.end) {
    const end = new Date(post.datetime.end);
    if (!isNaN(end.getTime())) {
      when += `–${formatTime(end)}`;
    }
  }

  if (post.datetime.recurring) {
    when = `Recurring · ${when}`;
  }

  if (post.datetime.cost) {
    when += ` · ${post.datetime.cost}`;
  }

  return when;
}

/**
 * Format a date's time as compact string: "9am", "10:30am", "2pm"
 */
function formatTime(d: Date): string {
  const hours = d.getHours();
  const minutes = d.getMinutes();
  const ampm = hours >= 12 ? 'pm' : 'am';
  const h = hours % 12 || 12;

  if (minutes === 0) return `${h}${ampm}`;
  return `${h}:${minutes.toString().padStart(2, '0')}${ampm}`;
}
