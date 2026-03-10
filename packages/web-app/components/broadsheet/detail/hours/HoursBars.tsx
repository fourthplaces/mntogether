import type { WeekSchedule } from '@/lib/broadsheet/hours';
import { DAY_NAMES, DAY_LETTERS, formatTimeShort, timeToHour, getTimeRange } from '@/lib/broadsheet/hours';

export function HoursBarsLarge({ week }: { week: WeekSchedule }) {
  const range = getTimeRange(week);
  const totalHours = range.max - range.min;
  return (
    <div className="hours-bars hours-bars--large">
      <div className="hours-bars__axis">
        {Array.from({ length: Math.ceil(totalHours / 2) + 1 }, (_, i) => {
          const h = range.min + i * 2;
          if (h > range.max) return null;
          return <span key={h} className="hours-bars__axis-label mono-sm">{formatTimeShort(`${h}:00`)}</span>;
        })}
      </div>
      {week.map((day, d) => (
        <div key={d} className="hours-bars__row">
          <span className="hours-bars__day mono-sm">{DAY_NAMES[d]}</span>
          {day ? (
            <div className="hours-bars__track">
              <div
                className="hours-bars__fill"
                style={{
                  left: `${((timeToHour(day.opens) - range.min) / totalHours * 100).toFixed(1)}%`,
                  width: `${((timeToHour(day.closes) - timeToHour(day.opens)) / totalHours * 100).toFixed(1)}%`,
                }}
              >
                <span className="hours-bars__fill-label mono-sm">
                  {formatTimeShort(day.opens)}{'\u2013'}{formatTimeShort(day.closes)}
                </span>
              </div>
            </div>
          ) : (
            <div className="hours-bars__closed">
              <span className="hours-bars__closed-text">Closed</span>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

export function HoursBarsSmall({ week }: { week: WeekSchedule }) {
  const range = getTimeRange(week);
  const totalHours = range.max - range.min;
  return (
    <div className="hours-bars hours-bars--small">
      {week.map((day, d) => (
        <div key={d} className="hours-bars__row">
          <span className="hours-bars__day mono-sm">{DAY_LETTERS[d]}</span>
          {day ? (
            <div className="hours-bars__track">
              <div
                className="hours-bars__fill"
                style={{
                  left: `${((timeToHour(day.opens) - range.min) / totalHours * 100).toFixed(1)}%`,
                  width: `${((timeToHour(day.closes) - timeToHour(day.opens)) / totalHours * 100).toFixed(1)}%`,
                }}
              />
            </div>
          ) : (
            <div className="hours-bars__closed" />
          )}
        </div>
      ))}
    </div>
  );
}
