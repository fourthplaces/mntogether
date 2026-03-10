interface WeatherDay {
  name: string;
  icon: string;
  hi: number;
  lo: number;
}

interface WeatherAlmanacProps {
  location: string;
  temp: number | string;
  condition: string;
  detail: string;
  days: WeatherDay[];
  advisory?: string;
}

export function WeatherAlmanac({ location, temp, condition, detail, days, advisory }: WeatherAlmanacProps) {
  const allTemps = days.flatMap(d => [d.hi, d.lo]);
  const minTemp = Math.min(...allTemps);
  const maxTemp = Math.max(...allTemps);
  const range = maxTemp - minTemp || 1;

  const barStyle = (lo: number, hi: number) => ({
    left: `${((lo - minTemp) / range) * 100}%`,
    width: `${Math.max(((hi - lo) / range) * 100, 4)}%`,
  });

  return (
    <div className="weather-almanac" data-debug="Widget.weatherAlmanac">
      <div className="weather-almanac__current">
        <div className="weather-almanac__location mono-sm">{location}</div>
        <div className="weather-almanac__temp">{temp}&deg;</div>
        <div className="weather-almanac__condition">{condition}</div>
        <div className="weather-almanac__detail">{detail}</div>
      </div>
      <div className="weather-almanac__table">
        <div className="weather-almanac__table-header">
          <span>Day</span>
          <span></span>
          <span>Range</span>
          <span style={{ textAlign: 'right' }}>High</span>
          <span style={{ textAlign: 'right' }}>Low</span>
        </div>
        {days.map((day, i) => (
          <div key={i} className="weather-almanac__row">
            <span className="weather-almanac__row-day">{day.name}</span>
            <span className="weather-almanac__row-icon">{day.icon}</span>
            <div className="weather-almanac__row-bar">
              <div className="weather-almanac__row-bar-track" />
              <div className="weather-almanac__row-bar-fill" style={barStyle(day.lo, day.hi)} />
            </div>
            <span className="weather-almanac__row-hi">{day.hi}&deg;</span>
            <span className="weather-almanac__row-lo">{day.lo}&deg;</span>
          </div>
        ))}
      </div>
      {advisory && <div className="weather-almanac__advisory">{advisory}</div>}
    </div>
  );
}
