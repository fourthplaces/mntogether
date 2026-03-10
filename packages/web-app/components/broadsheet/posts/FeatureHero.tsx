import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine } from '@/lib/broadsheet/display';

interface FeatureHeroProps {
  data: Post;
}

export function FeatureHero({ data: d }: FeatureHeroProps) {
  const c = 'feat-hero';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div
        className={`${c}__image newsprint-photo`}
        style={{ backgroundImage: `url('${d.media?.image || ''}')` }}
      />
      <div className={`${c}__content`}>
        {d.meta?.kicker && <div className={`${c}__kicker`}>{d.meta.kicker}</div>}
        <div className={`${c}__title`} dangerouslySetInnerHTML={{ __html: d.title }} />
        {d.deck && <div className={`${c}__deck`}>{d.deck}</div>}
        <div className={`${c}__meta mono-sm`}>{getMetaLine(d)}</div>
      </div>
    </div>
  );
}
