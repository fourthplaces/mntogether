import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine } from '@/lib/broadsheet/display';
import { MKicker, MTitle, MRichBody, MBody, MReadMore } from '@/lib/broadsheet/molecules';

interface FeatureStoryProps {
  data: Post;
}

export function FeatureStory({ data: d }: FeatureStoryProps) {
  const c = 'feat-story';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <div className={`${c}__rule`} />
      {d.meta?.kicker && <MKicker text={d.meta.kicker} prefix={c} pencilMark={d.pencilMark} />}
      <MTitle text={d.title} prefix={c} extra="condensed" />
      {d.deck && <div className={`${c}__deck`}>{d.deck}</div>}
      <div className={`${c}__byline mono-sm`}>{getMetaLine(d)}</div>
      {d.paragraphs
        ? <MRichBody paragraphs={d.paragraphs} prefix={c} cols={d.cols} dropCap={d.dropCap} />
        : <MBody text={d.body || ''} prefix={c} />}
      {d.pullQuote && (
        <div className={`${c}__pullquote`}>&ldquo;{d.pullQuote}&rdquo;</div>
      )}
      <MReadMore href={d.readMore || d.link?.url || '#'} text="Read full story" />
    </div>
  );
}
