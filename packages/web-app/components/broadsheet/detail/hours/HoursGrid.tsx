import type { WeekSchedule } from '@/lib/broadsheet/hours';
import { DAY_LETTERS, formatTimeShort, timeToHour, getTimeRange } from '@/lib/broadsheet/hours';

export function HoursGridLarge({ week }: { week: WeekSchedule }) {
  const range = getTimeRange(week);
  return (
    <div className="hours-grid hours-grid--large">
      <div className="hours-grid__header" />
      {DAY_LETTERS.map((d, i) => (
        <div key={i} className="hours-grid__header mono-sm">{d}</div>
      ))}
      {Array.from({ length: range.max - range.min }, (_, hi) => {
        const h = range.min + hi;
        return [
          <div key={`label-${h}`} className="hours-grid__hour-label mono-sm">{formatTimeShort(`${h}:00`)}</div>,
          ...Array.from({ length: 7 }, (_, d) => {
            const day = week[d];
            const isActive = day != null && h >= timeToHour(day.opens) && h < timeToHour(day.closes);
            return (
              <div key={`${h}-${d}`} className={`hours-grid__cell${isActive ? ' hours-grid__cell--active' : ''}`} />
            );
          }),
        ];
      })}
    </div>
  );
}

export function HoursGridSmall({ week }: { week: WeekSchedule }) {
  const range = getTimeRange(week);
  const blockStart = Math.floor(range.min / 3) * 3;
  const blockEnd = Math.ceil(range.max / 3) * 3;
  return (
    <div className="hours-grid hours-grid--small">
      <div className="hours-grid__header" />
      {DAY_LETTERS.map((d, i) => (
        <div key={i} className="hours-grid__header mono-sm">{d}</div>
      ))}
      {Array.from({ length: (blockEnd - blockStart) / 3 }, (_, bi) => {
        const h = blockStart + bi * 3;
        return [
          <div key={`label-${h}`} className="hours-grid__hour-label" />,
          ...Array.from({ length: 7 }, (_, d) => {
            const day = week[d];
            const isActive = day != null && (
              (h >= timeToHour(day.opens) && h < timeToHour(day.closes)) ||
              (h + 1 >= timeToHour(day.opens) && h + 1 < timeToHour(day.closes)) ||
              (h + 2 >= timeToHour(day.opens) && h + 2 < timeToHour(day.closes))
            );
            return (
              <div key={`${h}-${d}`} className={`hours-grid__cell${isActive ? ' hours-grid__cell--active' : ''}`} />
            );
          }),
        ];
      })}
    </div>
  );
}
