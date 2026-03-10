import type { WeekSchedule } from '@/lib/broadsheet/hours';
import { DAY_LETTERS, formatTimeShort, timeToHour, getTimeRange } from '@/lib/broadsheet/hours';

export function HoursHeatLarge({ week }: { week: WeekSchedule }) {
  const range = getTimeRange(week);
  return (
    <div className="hours-heat">
      <div className="hours-heat__header" />
      {DAY_LETTERS.map((d, i) => (
        <div key={i} className="hours-heat__header mono-sm">{d}</div>
      ))}
      {Array.from({ length: range.max - range.min }, (_, hi) => {
        const h = range.min + hi;
        return [
          <div key={`label-${h}`} className="hours-heat__hour-label mono-sm">{formatTimeShort(`${h}:00`)}</div>,
          ...Array.from({ length: 7 }, (_, d) => {
            const day = week[d];
            if (!day) return <div key={`${h}-${d}`} className="hours-heat__cell" />;

            const openH = timeToHour(day.opens);
            const closeH = timeToHour(day.closes);
            if (h < openH || h >= closeH) return <div key={`${h}-${d}`} className="hours-heat__cell" />;

            const midpoint = (openH + closeH) / 2;
            const distFromMid = Math.abs(h - midpoint);
            const maxDist = (closeH - openH) / 2;
            const isEdge = distFromMid > maxDist * 0.7;
            const isPeak = distFromMid < maxDist * 0.3;
            const level = isPeak ? 'peak' : isEdge ? 'edge' : 'active';

            return <div key={`${h}-${d}`} className={`hours-heat__cell hours-heat__cell--${level}`} />;
          }),
        ];
      })}
    </div>
  );
}

export function HoursHeatSmall({ week }: { week: WeekSchedule }) {
  let maxHours = 0;
  for (const day of week) {
    if (day) {
      maxHours = Math.max(maxHours, timeToHour(day.closes) - timeToHour(day.opens));
    }
  }
  return (
    <div className="hours-heat-bars">
      {week.map((day, d) => {
        const hours = day ? timeToHour(day.closes) - timeToHour(day.opens) : 0;
        const heightPct = maxHours > 0 ? Math.round(hours / maxHours * 100) : 0;
        return (
          <div key={d} className="hours-heat-bars__col">
            {day ? (
              <div className="hours-heat-bars__bar" style={{ height: `${heightPct}%` }} />
            ) : (
              <div className="hours-heat-bars__bar hours-heat-bars__bar--closed" />
            )}
            <div className="hours-heat-bars__day mono-sm">{DAY_LETTERS[d]}</div>
          </div>
        );
      })}
    </div>
  );
}
