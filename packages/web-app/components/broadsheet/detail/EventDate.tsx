/**
 * EventDate — calendar badge for event dates in the sidebar.
 *
 * Borrows the BulletinEvent calendar widget pattern (stacked month/day badge)
 * from the broadsheet homepage cards. Handles three cases:
 *   1. Single day — calendar badge + day-of-week + optional time
 *   2. Same-day start/end — treated as single day (no "through" silliness)
 *   3. Date range — start badge + "through" + end badge
 */

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const SHORT_MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const DAYS_OF_WEEK = ['Sunday', 'Monday', 'Tuesday', 'Wednesday',
  'Thursday', 'Friday', 'Saturday'];

function isSameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear()
    && a.getMonth() === b.getMonth()
    && a.getDate() === b.getDate();
}

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
 * Adapts the BulletinEvent `bul-event__cal` pattern for the detail sidebar.
 * Single-day: one badge + day-of-week. Range: two badges with "through" connector.
 */
export function EventDateA({ start, end, cost }: EventDateProps) {
  const startDate = new Date(start);
  const startMonth = SHORT_MONTHS[startDate.getMonth()];
  const startDay = startDate.getDate();
  const startDow = DAYS_OF_WEEK[startDate.getDay()];

  const endDate = end ? new Date(end) : null;
  const isRange = endDate && !isSameDay(startDate, endDate);

  return (
    <div className="event-date-a">
      <div className="event-date-a__row">
        {/* Start date badge */}
        <div className="event-date-a__cal">
          <span className="event-date-a__month mono-sm">{startMonth}</span>
          <span className="event-date-a__day">{startDay}</span>
        </div>

        <div className="event-date-a__info">
          <div className="event-date-a__dow condensed">{startDow}</div>
          {isRange && endDate && (
            <div className="event-date-a__range">
              through {SHORT_MONTHS[endDate.getMonth()]} {endDate.getDate()}
            </div>
          )}
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
 * Single line: "Apr 8" or "Apr 8 – Apr 12 · Free"
 */
export function EventDateB({ start, end, cost }: EventDateProps) {
  const startDate = new Date(start);
  const startLabel = `${SHORT_MONTHS[startDate.getMonth()]} ${startDate.getDate()}`;

  const endDate = end ? new Date(end) : null;
  const isRange = endDate && !isSameDay(startDate, endDate);
  const endLabel = isRange && endDate
    ? `${SHORT_MONTHS[endDate.getMonth()]} ${endDate.getDate()}`
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
