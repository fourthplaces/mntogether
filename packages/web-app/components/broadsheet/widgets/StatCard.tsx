interface StatCardProps {
  number: string;
  title: string;
  body: string;
}

export function StatCard({ number, title, body }: StatCardProps) {
  const c = 'stat-card';
  return (
    <div className={c} data-debug="Widget.statCard">
      <div className={`${c}__number condensed`}>{number}</div>
      <div className={`${c}__label`}>{title}</div>
      <div className={`${c}__context`}>{body}</div>
    </div>
  );
}
