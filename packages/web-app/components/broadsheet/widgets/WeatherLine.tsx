interface WeatherDay {
  name: string;
  icon: string;
  hi: number;
  lo: number;
}

interface WeatherLineProps {
  location: string;
  temp: number | string;
  condition: string;
  detail: string;
  days: WeatherDay[];
  advisory?: string;
}

function smoothPath(pts: { x: number; y: number }[]) {
  if (pts.length < 2) return '';
  const tension = 0.35;
  let path = `M ${pts[0].x},${pts[0].y}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i - 1] || pts[i];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[i + 2] || p2;
    const cp1x = p1.x + (p2.x - p0.x) * tension;
    const cp1y = p1.y + (p2.y - p0.y) * tension;
    const cp2x = p2.x - (p3.x - p1.x) * tension;
    const cp2y = p2.y - (p3.y - p1.y) * tension;
    path += ` C ${cp1x},${cp1y} ${cp2x},${cp2y} ${p2.x},${p2.y}`;
  }
  return path;
}

export function WeatherLine({ location, temp, condition, detail, days, advisory }: WeatherLineProps) {
  const allTemps = days.flatMap(d => [d.hi, d.lo]);
  const minTemp = Math.min(...allTemps);
  const maxTemp = Math.max(...allTemps);
  const range = maxTemp - minTemp || 1;

  const W = 440;
  const padL = 20, padR = 20;
  const padT = 32, padB = 28;
  const chartW = W - padL - padR;
  const chartH = 80;
  const H = padT + chartH + padB;

  const xOf = (i: number) => padL + (i / (days.length - 1)) * chartW;
  const yOf = (t: number) => padT + chartH - ((t - minTemp) / range) * chartH;

  const hiPts = days.map((d, i) => ({ x: xOf(i), y: yOf(d.hi) }));
  const loPts = days.map((d, i) => ({ x: xOf(i), y: yOf(d.lo) }));

  const hiPath = smoothPath(hiPts);
  const loPath = smoothPath(loPts);

  const loPathReversed = smoothPath([...loPts].reverse());
  const fillPath = hiPath +
    ` L ${loPts[loPts.length - 1].x},${loPts[loPts.length - 1].y}` +
    ` ${loPathReversed.replace(/^M /, 'L ')}` + ' Z';

  return (
    <div className="weather-line" data-debug="Widget.weatherLine">
      <div className="weather-line__info">
        <div className="weather-line__location mono-sm">{location}</div>
        <div className="weather-line__temp">{temp}&deg;</div>
        <div className="weather-line__condition">{condition}</div>
        <div className="weather-line__detail">{detail}</div>
        <div className="weather-line__legend">
          <div className="weather-line__legend-item">
            <div className="weather-line__legend-swatch" style={{ background: '#8C4026' }} />
            High
          </div>
          <div className="weather-line__legend-item">
            <div className="weather-line__legend-swatch" style={{ background: '#003865', borderTop: '1.5px dashed #003865', height: 0, width: 16 }} />
            Low
          </div>
        </div>
      </div>
      <div className="weather-line__chart">
        <svg viewBox={`0 0 ${W} ${H}`} xmlns="http://www.w3.org/2000/svg" style={{ overflow: 'visible' }}>
          <defs>
            <linearGradient id="fillGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#8C4026" stopOpacity={0.12} />
              <stop offset="100%" stopColor="#003865" stopOpacity={0.08} />
            </linearGradient>
          </defs>
          <path d={fillPath} fill="url(#fillGrad)" stroke="none" />
          <path d={hiPath} fill="none" stroke="#8C4026" strokeWidth={2} strokeLinecap="round" />
          <path d={loPath} fill="none" stroke="#003865" strokeWidth={1.5} strokeLinecap="round" strokeDasharray="4,3" />
          {days.map((day, i) => {
            const hx = hiPts[i].x, hy = hiPts[i].y;
            const lx = loPts[i].x, ly = loPts[i].y;
            return (
              <g key={i}>
                <line x1={hx} y1={padT} x2={hx} y2={padT + chartH} stroke="rgba(0,0,0,0.03)" strokeWidth={1} />
                <circle cx={hx} cy={hy} r={3.5} fill="#8C4026" stroke="var(--paper)" strokeWidth={1.5} />
                <text x={hx} y={hy - 9} textAnchor="middle" fill="#8C4026" fontFamily="'Feature Deck Condensed', sans-serif" fontWeight={700} fontSize={11}>{day.hi}&deg;</text>
                <text x={hx} y={hy - 22} textAnchor="middle" fontSize={12}>{day.icon}</text>
                <circle cx={lx} cy={ly} r={3} fill="#003865" stroke="var(--paper)" strokeWidth={1.5} />
                <text x={lx} y={ly + 14} textAnchor="middle" fill="#003865" fontFamily="'Geist Mono', monospace" fontSize={8} letterSpacing={0.3}>{day.lo}&deg;</text>
                <text x={hx} y={padT + chartH + 16} textAnchor="middle" fill="#78716c" fontFamily="'Geist Mono', monospace" fontSize={8} letterSpacing={0.8}>{day.name}</text>
              </g>
            );
          })}
        </svg>
      </div>
      {advisory && <div className="weather-line__advisory">{advisory}</div>}
    </div>
  );
}
