interface WeatherDay {
  name: string;
  icon: string;
  hi: number;
  lo: number;
}

interface WeatherForecastProps {
  location: string;
  temp: number | string;
  condition: string;
  detail: string;
  days: WeatherDay[];
  advisory?: string;
}

export function WeatherForecast({ location, temp, condition, detail, days, advisory }: WeatherForecastProps) {
  const c = 'weather-forecast';
  return (
    <div className={c} data-debug="Widget.weatherForecast">
      <div className={`${c}__current`}>
        <div className={`${c}__location mono-sm`}>{location}</div>
        <div className={`${c}__temp`}>{temp}&deg;</div>
        <div className={`${c}__condition`}>{condition}</div>
        <div className={`${c}__detail`}>{detail}</div>
      </div>
      <div className={`${c}__days`}>
        {days.map((day, i) => (
          <div key={i} className={`${c}__day`}>
            <div className={`${c}__day-name`}>{day.name}</div>
            <div className={`${c}__day-icon`}>{day.icon}</div>
            <div className={`${c}__day-hi`}>{day.hi}&deg;</div>
            <div className={`${c}__day-lo`}>Lo {day.lo}&deg;</div>
          </div>
        ))}
      </div>
      {advisory && <div className={`${c}__advisory`}>{advisory}</div>}
    </div>
  );
}
