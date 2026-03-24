import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine } from '@/lib/broadsheet/display';
import { MTitle, MRichBody, MBody, MReadMore } from '@/lib/broadsheet/molecules';

interface FeatureEditorialProps {
  data: Post;
}

export function FeatureEditorial({ data: d }: FeatureEditorialProps) {
  const c = 'feat-editorial';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTitle text={d.title} prefix={c} />
      <div className={`${c}__byline mono-sm`}>{getMetaLine(d)}</div>
      {d.paragraphs
        ? <MRichBody paragraphs={d.paragraphs} prefix={c} cols={d.cols} dropCap={d.dropCap} />
        : <MBody text={d.body || ''} prefix={c} />}
      <MReadMore href={d.readMore || d.link?.url || '#'} text="Read full story" />
    </div>
  );
}
