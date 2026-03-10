import type { Post } from '@/lib/broadsheet/types';
import { getDetailsLine } from '@/lib/broadsheet/display';
import { MTagline } from '@/lib/broadsheet/molecules';

interface DigestSpotlightProps {
  data: Post;
}

export function DigestSpotlight({ data: d }: DigestSpotlightProps) {
  const c = 'dig-local';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div className={`${c}__label mono-sm`}>Support Local</div>
      <div className={`${c}__name`} dangerouslySetInnerHTML={{ __html: d.title }} />
      {d.tagline && <MTagline text={d.tagline} prefix={c} />}
      {getDetailsLine(d) && (
        <div className={`${c}__detail mono-sm`}>{getDetailsLine(d)}</div>
      )}
    </div>
  );
}
