import type { Post } from '@/lib/broadsheet/types';
import { getDetailsLine } from '@/lib/broadsheet/display';
import { MTag, MTitle, MTagline, MBody } from '@/lib/broadsheet/molecules';

interface BulletinSpotlightProps {
  data: Post;
}

export function BulletinSpotlight({ data: d }: BulletinSpotlightProps) {
  const c = 'bul-local';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <MTag text="Local" prefix={c} />
      <MTitle text={d.title} prefix={c} />
      {d.tagline && <MTagline text={d.tagline} prefix={c} />}
      <MBody text={d.body || ''} prefix={c} clamp={d.clamp || 4} />
      {getDetailsLine(d) && (
        <div className={`${c}__details mono-sm`}>{getDetailsLine(d)}</div>
      )}
    </div>
  );
}
