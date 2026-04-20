import type { Post } from '@/lib/broadsheet/types';
import { MTag, MInlineTitle } from '@/lib/broadsheet/molecules';

interface TickerEventProps {
  data: Post;
}

export function TickerEvent({ data: d }: TickerEventProps) {
  const c = 'tick-event';
  // Meta: Address • Date, Time (right-side mono-sm text)
  const metaParts: string[] = [];
  if (d.location?.address) metaParts.push(d.location.address);
  if (d.when) {
    metaParts.push(d.when);
  } else if (d.day && d.month) {
    metaParts.push(`${d.day} ${d.month}`);
  }
  const meta = metaParts.join(' \u00b7 ');

  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text="Event" prefix={c} />
      <MInlineTitle text={d.title} prefix={c} />
      {meta && <span className={`${c}__meta mono-sm`}>{meta}</span>}
    </div>
  );
}
