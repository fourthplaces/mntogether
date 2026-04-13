import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine } from '@/lib/broadsheet/display';
import { MTag, MKicker, MTitle, MMeta, MRichBody, MBody, MReadMore } from '@/lib/broadsheet/molecules';

interface BulletinStoryProps {
  data: Post;
}

export function BulletinStory({ data: d }: BulletinStoryProps) {
  const c = 'bul-story';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text="Story" prefix={c} />
      {d.meta?.kicker && <MKicker text={d.meta.kicker} prefix={c} />}
      <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
      <MMeta text={getMetaLine(d)} prefix={c} />
      {d.paragraphs
        ? <MRichBody paragraphs={d.paragraphs} prefix={c} cols={d.cols} dropCap={d.dropCap} />
        : <MBody text={d.body || ''} prefix={c} />}
      <MReadMore href={d.readMore || d.link?.url || '#'} text="Read full story" />
    </div>
  );
}
