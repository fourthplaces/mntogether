import type { ResourceItem } from '@/lib/broadsheet/detail-types';

interface ListProps {
  items: string[];
  ordered?: boolean;
}

export function ListA({ items, ordered = false }: ListProps) {
  const Tag = ordered ? 'ol' : 'ul';
  return (
    <Tag className="list-a">
      {items.map((item, i) => <li key={i}>{item}</li>)}
    </Tag>
  );
}

export function ListB({ items, ordered = false }: ListProps) {
  const Tag = ordered ? 'ol' : 'ul';
  return (
    <Tag className="list-b">
      {items.map((item, i) => <li key={i}>{item}</li>)}
    </Tag>
  );
}

export function ResourceListA({ items }: { items: ResourceItem[] }) {
  return (
    <ul className="list-a list-a--resource">
      {items.map((item, i) => (
        <li key={i}><strong>{item.name}</strong> {'\u00b7'} {item.detail}</li>
      ))}
    </ul>
  );
}

export function ResourceListB({ items }: { items: ResourceItem[] }) {
  return (
    <ul className="list-b">
      {items.map((item, i) => (
        <li key={i}><strong>{item.name}</strong> {'\u00b7'} {item.detail}</li>
      ))}
    </ul>
  );
}
