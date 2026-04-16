interface ResourceBarItem {
  number: string;
  text: string;
  url?: string;
}

interface ResourceBarProps {
  label: string;
  items: ResourceBarItem[];
}

/** Digits-and-dashes pattern: detect phone numbers like "612-348-3000" or "211" */
function looksLikePhone(s: string): boolean {
  return /^[\d\s()+-]+$/.test(s.replace(/-/g, ""));
}

/** Strip everything except digits from a phone string for tel: href */
function toTelHref(s: string): string {
  return `tel:${s.replace(/[^\d+]/g, "")}`;
}

export function ResourceBar({ label, items }: ResourceBarProps) {
  return (
    <div className="resource-bar" data-debug="Widget.resourceBar">
      <span className="rb-label mono-sm">{label}</span>
      {items.map((item, i) => {
        const href = item.url
          ? item.url
          : looksLikePhone(item.number)
            ? toTelHref(item.number)
            : undefined;

        return (
          <span key={i} className="rb-item mono-sm">
            {href ? (
              <a href={href}>
                <strong>{item.number}</strong> {item.text}
              </a>
            ) : (
              <>
                <strong>{item.number}</strong> {item.text}
              </>
            )}
          </span>
        );
      })}
    </div>
  );
}
