/**
 * EventDate — calendar badge for event dates in the sidebar.
 *
 * Borrows the BulletinEvent calendar widget pattern (stacked month/day badge)
 * from the broadsheet homepage cards. Handles three cases:
 *   1. Single day — calendar badge + day-of-week + optional time
 *   2. Same-day start/end — treated as single day (no "through" silliness)
 *   3. Date range — start badge + "through" + end badge
 *
 * All dates displayed in Minnesota timezone (America/Chicago).
 */

import { extractEventParts, formatEventDate } from '@/lib/broadsheet/dates';

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

interface EventDateProps {
  start: string;
  end?: string | null;
  cost?: string | null;
}

/**
 * EventDate A — Calendar badge (sidebar).
 * Single-day: one badge + full date with relative hint.
 * Range: badge + "Saturday, March 15 – Sunday, March 16"
 */
export function EventDateA({ start, end, cost }: EventDateProps) {
  const startParts = extractEventParts(start);
  if (!startParts) return null;

  const dateLabel = formatEventDate(start, end);

  return (
    <div className="event-date-a">
      <div className="event-date-a__row">
        {/* Start date badge */}
        <div className="event-date-a__cal">
          <span className="event-date-a__month mono-sm">{startParts.month}</span>
          <span className="event-date-a__day">{startParts.day}</span>
        </div>

        <div className="event-date-a__info">
          <div className="event-date-a__dow condensed">{dateLabel}</div>
          {cost && (
            <div className="event-date-a__cost mono-sm">{cost}</div>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * EventDate B — Compact inline.
 * Single line: "Mar 15" or "Mar 15 – Mar 17 · Free"
 */
export function EventDateB({ start, end, cost }: EventDateProps) {
  const startParts = extractEventParts(start);
  if (!startParts) return null;

  const startLabel = `${startParts.month.charAt(0)}${startParts.month.slice(1).toLowerCase()} ${startParts.day}`;

  const endParts = end ? extractEventParts(end) : null;
  const isRange = endParts
    && (endParts.month !== startParts.month || endParts.day !== startParts.day);
  const endLabel = isRange && endParts
    ? `${endParts.month.charAt(0)}${endParts.month.slice(1).toLowerCase()} ${endParts.day}`
    : null;

  return (
    <div className="event-date-b">
      <span className="event-date-b__range">
        {startLabel}
        {endLabel && <> &ndash; {endLabel}</>}
      </span>
      {cost && (
        <>
          <span className="event-date-b__sep"> &middot; </span>
          <span className="event-date-b__cost">{cost}</span>
        </>
      )}
    </div>
  );
}
