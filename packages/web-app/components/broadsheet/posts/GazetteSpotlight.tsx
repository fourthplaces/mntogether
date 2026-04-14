import type { Post } from '@/lib/broadsheet/types';
import { getDetailsLine } from '@/lib/broadsheet/display';
import { MTag, MTitle, MTagline, MBody } from '@/lib/broadsheet/molecules';

interface GazetteSpotlightProps {
  data: Post;
}

export function GazetteSpotlight({ data: d }: GazetteSpotlightProps) {
  const c = 'gaz-local';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text="Local" prefix={c} />
      <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
      {d.tagline && <MTagline text={d.tagline} prefix={c} />}
      <MBody text={d.body || ''} prefix={c} clamp={d.clamp ?? 4} />
      {getDetailsLine(d) && (
        <div className={`${c}__details mono-sm`}>{getDetailsLine(d)}</div>
      )}
    </div>
  );
}
