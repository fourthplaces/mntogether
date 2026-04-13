import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine } from '@/lib/broadsheet/display';
import { MKicker, MTitle, MMeta } from '@/lib/broadsheet/molecules';

interface DigestStoryProps {
  data: Post;
}

export function DigestStory({ data: d }: DigestStoryProps) {
  const c = 'dig-story';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      {d.meta?.kicker && <MKicker text={d.meta.kicker} prefix={c} small />}
      <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
      <MMeta text={getMetaLine(d)} prefix={c} small />
    </div>
  );
}
