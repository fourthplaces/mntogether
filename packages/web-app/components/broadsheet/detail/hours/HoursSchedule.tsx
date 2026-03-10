import type { WeekSchedule } from '@/lib/broadsheet/hours';
import { DAY_NAMES, DAY_FULL, formatTime12h, formatTimeShort, groupDays, closedDaysLabel } from '@/lib/broadsheet/hours';

export function HoursScheduleLarge({ week }: { week: WeekSchedule }) {
  const groups = groupDays(week);
  const closed = closedDaysLabel(week);
  return (
    <div className="hours-schedule hours-schedule--large">
      {groups.map((g, i) => {
        const dayLabel = g.startDay === g.endDay
          ? DAY_FULL[g.startDay]
          : `${DAY_FULL[g.startDay]}\u2013${DAY_FULL[g.endDay]}`;
        return (
          <div key={i} className="hours-schedule__row">
            <span className="hours-schedule__days condensed">{dayLabel}</span>
            <span className="hours-schedule__times mono-sm">{formatTime12h(g.opens)} {'\u2013'} {formatTime12h(g.closes)}</span>
          </div>
        );
      })}
      {closed && <div className="hours-schedule__closed">{closed}</div>}
    </div>
  );
}

export function HoursScheduleSmall({ week }: { week: WeekSchedule }) {
  const groups = groupDays(week);
  return (
    <div className="hours-schedule hours-schedule--small">
      {groups.map((g, i) => {
        const dayLabel = g.startDay === g.endDay
          ? DAY_NAMES[g.startDay]
          : `${DAY_NAMES[g.startDay]}\u2013${DAY_NAMES[g.endDay]}`;
        return (
          <span key={i} className="hours-schedule__row">
            <span className="hours-schedule__days condensed">{dayLabel}</span>{' '}
            <span className="hours-schedule__times mono-sm">{formatTimeShort(g.opens)}{'\u2013'}{formatTimeShort(g.closes)}</span>
          </span>
        );
      })}
    </div>
  );
}
