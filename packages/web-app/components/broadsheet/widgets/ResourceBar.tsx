interface ResourceBarItem {
  number: string;
  text: string;
}

interface ResourceBarProps {
  label: string;
  items: ResourceBarItem[];
}

export function ResourceBar({ label, items }: ResourceBarProps) {
  return (
    <div className="resource-bar" data-debug="Widget.resourceBar">
      <span className="rb-label mono-sm">{label}</span>
      {items.map((item, i) => (
        <span key={i} className="rb-item mono-sm">
          <strong>{item.number}</strong> {item.text}
        </span>
      ))}
    </div>
  );
}
