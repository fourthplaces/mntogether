import type { Post } from '@/lib/broadsheet/types';
import { MTag } from '@/lib/broadsheet/molecules';

interface TickerEventProps {
  data: Post;
}

export function TickerEvent({ data: d }: TickerEventProps) {
  const c = 'tick-event';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text="Event" prefix={c} />
      <span className={`${c}__title`} dangerouslySetInnerHTML={{ __html: d.title }} />
      <span className={`${c}__meta mono-sm`}>{d.day || ''} {d.month || ''}</span>
    </div>
  );
}
