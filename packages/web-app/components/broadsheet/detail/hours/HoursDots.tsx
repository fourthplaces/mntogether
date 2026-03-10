import type { WeekSchedule } from '@/lib/broadsheet/hours';
import { DAY_NAMES, DAY_LETTERS, DAY_FULL, formatTimeShort, formatTime12h } from '@/lib/broadsheet/hours';

export function HoursDotsLarge({ week }: { week: WeekSchedule }) {
  return (
    <div className="hours-dots hours-dots--large">
      {week.map((day, d) => (
        <div key={d} className="hours-dots__col">
          <div className="hours-dots__day mono-sm">{DAY_NAMES[d]}</div>
          <div className={`hours-dots__dot hours-dots__dot--${day ? 'open' : 'closed'}`} />
          {day ? (
            <div className="hours-dots__time mono-sm">
              {formatTimeShort(day.opens)}<br />{'\u2013'}<br />{formatTimeShort(day.closes)}
            </div>
          ) : (
            <div className="hours-dots__time" style={{ fontStyle: 'italic' }}>Closed</div>
          )}
        </div>
      ))}
    </div>
  );
}

export function HoursDotsSmall({ week }: { week: WeekSchedule }) {
  return (
    <div className="hours-dots hours-dots--small">
      {week.map((day, d) => (
        <div key={d} className="hours-dots__col">
          <div className="hours-dots__day mono-sm">{DAY_LETTERS[d]}</div>
          <div
            className={`hours-dots__dot hours-dots__dot--${day ? 'open' : 'closed'}`}
            title={day
              ? `${DAY_FULL[d]}: ${formatTime12h(day.opens)} \u2013 ${formatTime12h(day.closes)}`
              : `${DAY_FULL[d]}: Closed`
            }
          />
        </div>
      ))}
    </div>
  );
}
