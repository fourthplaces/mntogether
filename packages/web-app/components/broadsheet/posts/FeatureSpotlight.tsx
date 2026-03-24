import type { Post } from '@/lib/broadsheet/types';
import { getDetailsLine } from '@/lib/broadsheet/display';
import { MTagline, MBody } from '@/lib/broadsheet/molecules';

interface FeatureSpotlightProps {
  data: Post;
}

export function FeatureSpotlight({ data: d }: FeatureSpotlightProps) {
  // Person profile variant
  if (d.tags?.includes('person') && d.person) {
    const c = 'feat-profile';
    return (
      <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
        <div
          className={`${c}__photo`}
          style={{ backgroundImage: `url('${d.person.photo || d.media?.image || ''}')` }}
        />
        <div className={`${c}__content`}>
          <div className={`${c}__label mono-sm`}>{d.meta?.kicker || 'Community Voice'}</div>
          <div className={`${c}__name condensed`}>{d.person.name || d.title}</div>
          <div className={`${c}__role`}>{d.person.role || ''}</div>
          {d.person.quote && (
            <div className={`${c}__quote`}>&ldquo;{d.person.quote}&rdquo;</div>
          )}
          {(d.person.bio || d.body) && (
            <div className={`${c}__bio`}>{d.person.bio || d.body}</div>
          )}
        </div>
      </div>
    );
  }

  // Business/place spotlight
  const c = 'spotlight-local';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <div className={`${c}__label mono-sm`}>Support Local</div>
      <div className={`${c}__name condensed`} dangerouslySetInnerHTML={{ __html: d.title }} />
      {d.tagline && <MTagline text={d.tagline} prefix={c} />}
      <MBody text={d.body || ''} prefix={c} clamp={d.clamp || 4} />
      {getDetailsLine(d) && (
        <span className={`${c}__address mono-sm`}>{getDetailsLine(d)}</span>
      )}
    </div>
  );
}
