/* ═══════════════════════════════════════════════
   HOURS UTILITIES
   Shared schedule data helpers for Hours widgets.
   ═══════════════════════════════════════════════ */

export const DAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
export const DAY_FULL = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
export const DAY_LETTERS = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];

export interface ScheduleEntry {
  day_of_week: number;
  opens_at: string;
  closes_at: string;
}

export interface DayHours {
  opens: string;
  closes: string;
}

/** 7-element array (Sun=0 .. Sat=6), null = closed */
export type WeekSchedule = (DayHours | null)[];

export function formatTime12h(time24: string): string {
  const [h, m] = time24.split(':').map(Number);
  const suffix = h >= 12 ? 'PM' : 'AM';
  const h12 = h % 12 || 12;
  return m === 0 ? `${h12} ${suffix}` : `${h12}:${m.toString().padStart(2, '0')} ${suffix}`;
}

export function formatTimeShort(time24: string): string {
  const [h, m] = time24.split(':').map(Number);
  const suffix = h >= 12 ? 'p' : 'a';
  const h12 = h % 12 || 12;
  return m === 0 ? `${h12}${suffix}` : `${h12}:${m.toString().padStart(2, '0')}${suffix}`;
}

export function timeToHour(time24: string): number {
  return parseInt(time24.split(':')[0], 10);
}

/** Turn schedule records into a 7-element array */
export function normalizeSchedule(schedules: ScheduleEntry[]): WeekSchedule {
  const week: WeekSchedule = [null, null, null, null, null, null, null];
  for (const s of schedules) {
    if (s.day_of_week != null && s.opens_at && s.closes_at) {
      week[s.day_of_week] = {
        opens: s.opens_at.slice(0, 5),
        closes: s.closes_at.slice(0, 5),
      };
    }
  }
  return week;
}

/** Find min open / max close across all days (for axis ranges) */
export function getTimeRange(week: WeekSchedule): { min: number; max: number } {
  let min = 24, max = 0;
  for (const day of week) {
    if (day) {
      min = Math.min(min, timeToHour(day.opens));
      max = Math.max(max, timeToHour(day.closes));
    }
  }
  return { min: Math.max(0, min - 1), max: Math.min(24, max + 1) };
}

export interface DayGroup {
  startDay: number;
  endDay: number;
  opens: string;
  closes: string;
}

/** Group consecutive days with identical hours */
export function groupDays(week: WeekSchedule): DayGroup[] {
  const groups: DayGroup[] = [];
  let current: DayGroup | null = null;
  for (let i = 0; i < 7; i++) {
    const day = week[i];
    if (day && current && current.opens === day.opens && current.closes === day.closes) {
      current.endDay = i;
    } else if (day) {
      current = { startDay: i, endDay: i, opens: day.opens, closes: day.closes };
      groups.push(current);
    } else {
      current = null;
    }
  }
  return groups;
}

export function closedDays(week: WeekSchedule): number[] {
  return week.map((d, i) => d ? null : i).filter((i): i is number => i !== null);
}

export function closedDaysLabel(week: WeekSchedule): string {
  const closed = closedDays(week);
  if (closed.length === 0) return '';
  return 'Closed ' + closed.map(i => DAY_NAMES[i]).join(', ');
}
