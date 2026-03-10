import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine } from '@/lib/broadsheet/display';
import { MTag } from '@/lib/broadsheet/molecules';

interface TickerStoryProps {
  data: Post;
}

export function TickerStory({ data: d }: TickerStoryProps) {
  const c = 'tick-story';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <MTag text="Story" prefix={c} />
      <span className={`${c}__title`} dangerouslySetInnerHTML={{ __html: d.title }} />
      <span className={`${c}__meta mono-sm`}>{getMetaLine(d)}</span>
    </div>
  );
}
