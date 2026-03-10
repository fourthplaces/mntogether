interface KickerAProps {
  tags: string[];
  colors?: Record<string, string>;
}

export function KickerA({ tags, colors = {} }: KickerAProps) {
  return (
    <div className="kicker-a mono-md">
      {tags.map((t, i) => (
        <span key={t}>
          <a href="#" style={{ color: colors[t] || 'var(--sienna)' }}>{t}</a>
          {i < tags.length - 1 && <span className="sep">{'\u00b7'}</span>}
        </span>
      ))}
    </div>
  );
}

interface KickerBProps {
  primary: string;
  secondary?: string[];
  color?: string;
}

export function KickerB({ primary, secondary = [], color = 'var(--deep-forest)' }: KickerBProps) {
  return (
    <div className="kicker-b">
      <a href="#" className="kicker-b__primary mono-md" style={{ borderTopColor: color, color }}>{primary}</a>
      {secondary.length > 0 && (
        <div className="kicker-b__secondary">
          {secondary.map(t => (
            <a key={t} href="#" className="kicker-b__pill mono-sm">{t}</a>
          ))}
        </div>
      )}
    </div>
  );
}
