interface WeatherDay {
  name: string;
  icon: string;
  hi: number;
  lo: number;
}

interface WeatherThermoProps {
  location: string;
  temp: number | string;
  condition: string;
  detail: string;
  days: WeatherDay[];
  advisory?: string;
}

function barColor(lo: number, hi: number) {
  const avg = (lo + hi) / 2;
  if (avg < 10) return 'rgba(0, 56, 101, 0.35)';
  if (avg < 32) return 'rgba(0, 56, 101, 0.25)';
  if (avg < 55) return 'rgba(0, 110, 100, 0.3)';
  if (avg < 75) return 'rgba(192, 148, 42, 0.35)';
  return 'rgba(140, 64, 38, 0.35)';
}

export function WeatherThermo({ location, temp, condition, detail, days, advisory }: WeatherThermoProps) {
  const allTemps = days.flatMap(d => [d.hi, d.lo]);
  const minTemp = Math.min(...allTemps);
  const maxTemp = Math.max(...allTemps);
  const range = maxTemp - minTemp || 1;

  const scaleMin = Math.floor((minTemp - range * 0.08) / 5) * 5;
  const scaleMax = Math.ceil((maxTemp + range * 0.08) / 5) * 5;
  const scaleRange = scaleMax - scaleMin || 1;

  const barLeft = (lo: number) => ((lo - scaleMin) / scaleRange) * 100;
  const barWidth = (lo: number, hi: number) => Math.max(((hi - lo) / scaleRange) * 100, 3);

  const step = scaleRange > 60 ? 20 : scaleRange > 30 ? 10 : 5;
  const scaleLabels: { temp: number; pct: number }[] = [];
  for (let t = Math.ceil(scaleMin / step) * step; t <= scaleMax; t += step) {
    scaleLabels.push({ temp: t, pct: ((t - scaleMin) / scaleRange) * 100 });
  }

  return (
    <div className="weather-thermo" data-debug="Widget.weatherThermo">
      <div className="weather-thermo__top">
        <div className="weather-thermo__current">
          <div className="weather-thermo__temp">{temp}&deg;</div>
          <div className="weather-thermo__condition-wrap">
            <div className="weather-thermo__condition">{condition}</div>
            <div className="weather-thermo__detail-line">{detail}</div>
          </div>
        </div>
        <div className="weather-thermo__location">{location}</div>
      </div>
      <div className="weather-thermo__scale" style={{ position: 'relative', height: 18 }}>
        {scaleLabels.map((s, i) => (
          <span key={i} style={{ position: 'absolute', left: `${s.pct}%`, transform: 'translateX(-50%)' }}>{s.temp}&deg;</span>
        ))}
      </div>
      <div className="weather-thermo__rows">
        {days.map((day, i) => (
          <div key={i} className="weather-thermo__row">
            <div className="weather-thermo__row-label">
              <span className="weather-thermo__row-icon">{day.icon}</span>
              <span className="weather-thermo__row-day">{day.name}</span>
            </div>
            <div className="weather-thermo__row-track">
              <div className="weather-thermo__row-track-bg" />
              <div
                className="weather-thermo__row-bar"
                style={{
                  left: `${barLeft(day.lo)}%`,
                  width: `${barWidth(day.lo, day.hi)}%`,
                  background: barColor(day.lo, day.hi),
                }}
              >
                <span className="weather-thermo__row-bar-lo">{day.lo}&deg;</span>
                <span className="weather-thermo__row-bar-hi">{day.hi}&deg;</span>
              </div>
            </div>
          </div>
        ))}
      </div>
      {advisory && <div className="weather-thermo__advisory">{advisory}</div>}
    </div>
  );
}
