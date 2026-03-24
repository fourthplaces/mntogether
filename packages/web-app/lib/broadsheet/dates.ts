/**
 * Central date formatting for broadsheet display.
 *
 * Rules:
 *  - Always Minnesota timezone (America/Chicago)
 *  - Never show hours/minutes/seconds for publication dates
 *  - Relative language for near dates: "Today", "Yesterday", "Tomorrow"
 *  - Human-readable month names, never raw ISO strings
 */

const TZ = 'America/Chicago';

// ---------------------------------------------------------------------------
// Internal: extract date parts in Minnesota timezone
// ---------------------------------------------------------------------------

interface DateParts {
  year: number;
  month: number; // 1-based
  day: number;
  weekday: string; // "Monday", "Tuesday", ...
}

function toMN(d: Date): DateParts {
  const fmt = new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    weekday: 'long',
  });
  const parts = fmt.formatToParts(d);
  const get = (type: string) => parts.find((p) => p.type === type)?.value ?? '';
  return {
    year: parseInt(get('year'), 10),
    month: parseInt(get('month'), 10),
    day: parseInt(get('day'), 10),
    weekday: get('weekday'),
  };
}

function nowMN(): DateParts {
  return toMN(new Date());
}

/** Days between two MN dates (positive = future, negative = past) */
function diffDays(a: DateParts, b: DateParts): number {
  const da = Date.UTC(a.year, a.month - 1, a.day);
  const db = Date.UTC(b.year, b.month - 1, b.day);
  return Math.round((db - da) / (1000 * 60 * 60 * 24));
}

function parse(iso: string): Date | null {
  const d = new Date(iso);
  return isNaN(d.getTime()) ? null : d;
}

// ---------------------------------------------------------------------------
// Formatters — used by Intl
// ---------------------------------------------------------------------------

const fmtMonthShort = new Intl.DateTimeFormat('en-US', { timeZone: TZ, month: 'short' });
const fmtMonthLong  = new Intl.DateTimeFormat('en-US', { timeZone: TZ, month: 'long' });
const fmtDowShort   = new Intl.DateTimeFormat('en-US', { timeZone: TZ, weekday: 'short' });
const fmtDowLong    = new Intl.DateTimeFormat('en-US', { timeZone: TZ, weekday: 'long' });

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * For article meta line (publication/update date).
 * "Today" · "Yesterday" · "3 days ago" · "2 weeks ago" · "March 15"
 * Omits year if same as current year.
 */
export function formatPostDate(iso: string): string {
  const d = parse(iso);
  if (!d) return '';

  const mn = toMN(d);
  const today = nowMN();
  const ago = diffDays(mn, today); // positive = d is in the past

  if (ago === 0) return 'Today';
  if (ago === 1) return 'Yesterday';
  if (ago >= 2 && ago <= 6) return `${ago} days ago`;
  if (ago >= 7 && ago <= 13) return '1 week ago';
  if (ago >= 14 && ago <= 27) return `${Math.floor(ago / 7)} weeks ago`;

  // Absolute date
  const month = fmtMonthLong.format(d);
  if (mn.year === today.year) return `${month} ${mn.day}`;
  return `${month} ${mn.day}, ${mn.year}`;
}

/**
 * For event circle badge on broadsheet cards.
 * "Today!" · "Tomorrow" · "Sat" · "Mar 15"
 */
export function formatEventLabel(iso: string): string {
  const d = parse(iso);
  if (!d) return '';

  const mn = toMN(d);
  const today = nowMN();
  const diff = diffDays(today, mn); // positive = future

  if (diff === 0) return 'Today!';
  if (diff === 1) return 'Tomorrow';
  if (diff >= 2 && diff <= 6) return fmtDowShort.format(d);
  return `${fmtMonthShort.format(d)} ${mn.day}`;
}

/**
 * Extract month/day/dow for event card rendering.
 * month: "MAR", day: "15", dow: "Sat"
 */
export function extractEventParts(iso: string): {
  month: string;
  day: string;
  dow: string;
} | null {
  const d = parse(iso);
  if (!d) return null;

  const mn = toMN(d);
  return {
    month: fmtMonthShort.format(d).toUpperCase(),
    day: mn.day.toString(),
    dow: fmtDowShort.format(d),
  };
}

/**
 * For event sidebar detail: "Saturday, March 15" or with year if different.
 * Range: "Saturday, March 15 – Sunday, March 16"
 */
export function formatEventDate(startIso: string, endIso?: string | null): string {
  const s = parse(startIso);
  if (!s) return '';

  const sMN = toMN(s);
  const today = nowMN();

  const formatOne = (d: Date, p: DateParts): string => {
    const dow = fmtDowLong.format(d);
    const month = fmtMonthLong.format(d);
    const yearSuffix = p.year !== today.year ? `, ${p.year}` : '';
    return `${dow}, ${month} ${p.day}${yearSuffix}`;
  };

  let result = formatOne(s, sMN);

  // Add relative hint
  const diff = diffDays(today, sMN);
  if (diff === 0) result += ' (Today)';
  else if (diff === 1) result += ' (Tomorrow)';

  if (endIso) {
    const e = parse(endIso);
    if (e) {
      const eMN = toMN(e);
      // Only show range if different day
      if (eMN.year !== sMN.year || eMN.month !== sMN.month || eMN.day !== sMN.day) {
        result += ` – ${formatOne(e, eMN)}`;
      }
    }
  }

  return result;
}

/**
 * For event when line on cards: "Sat 10am–2pm · Free"
 * Uses event start/end times in MN timezone.
 */
export function formatEventWhen(
  startIso: string,
  endIso?: string | null,
  cost?: string | null,
  recurring?: boolean
): string {
  const s = parse(startIso);
  if (!s) return '';

  const dow = fmtDowShort.format(s);
  const startTime = formatCompactTime(s);

  let when = `${dow} ${startTime}`;

  if (endIso) {
    const e = parse(endIso);
    if (e) when += `–${formatCompactTime(e)}`;
  }

  if (recurring) when = `Recurring · ${when}`;
  if (cost) when += ` · ${cost}`;

  return when;
}

/**
 * For deadlines: "Friday, March 14, 2026"
 * Always includes year (deadlines are specific).
 */
export function formatDeadline(iso: string): string {
  const d = parse(iso);
  if (!d) return iso; // fallback to raw string

  // If it looks like a date-only string (YYYY-MM-DD), parse as MN noon to avoid timezone shift
  const dateOnly = /^\d{4}-\d{2}-\d{2}$/.test(iso);
  const target = dateOnly ? new Date(iso + 'T12:00:00-05:00') : d;

  const mn = toMN(target);
  const dow = fmtDowLong.format(target);
  const month = fmtMonthLong.format(target);
  return `${dow}, ${month} ${mn.day}, ${mn.year}`;
}

/**
 * For the "Updated" line on reference cards: "Updated March 2026"
 */
export function formatUpdatedMonth(iso: string): string {
  const d = parse(iso);
  if (!d) return '';

  const mn = toMN(d);
  const month = fmtMonthLong.format(d);
  return `Updated ${month} ${mn.year}`;
}

// ---------------------------------------------------------------------------
// Internal: compact time formatting (for event times only)
// ---------------------------------------------------------------------------

/** "9am", "10:30am", "2pm" — in MN timezone */
function formatCompactTime(d: Date): string {
  const timeParts = new Intl.DateTimeFormat('en-US', {
    timeZone: TZ,
    hour: 'numeric',
    minute: '2-digit',
    hour12: true,
  }).formatToParts(d);

  const get = (type: string) => timeParts.find((p) => p.type === type)?.value ?? '';
  const hour = get('hour');
  const minute = get('minute');
  const period = get('dayPeriod').toLowerCase();

  if (minute === '00') return `${hour}${period}`;
  return `${hour}:${minute}${period}`;
}
