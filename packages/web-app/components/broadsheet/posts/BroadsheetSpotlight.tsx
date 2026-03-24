import type { Post } from '@/lib/broadsheet/types';
import { getDetailsLine } from '@/lib/broadsheet/display';
import { MTagline, MBody } from '@/lib/broadsheet/molecules';

interface BroadsheetSpotlightProps {
  data: Post;
}

export function BroadsheetSpotlight({ data: d }: BroadsheetSpotlightProps) {
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
