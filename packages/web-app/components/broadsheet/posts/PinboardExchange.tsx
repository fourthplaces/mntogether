import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine, getTagLabel } from '@/lib/broadsheet/display';
import { MTag, MTitle, MBody, MStatus } from '@/lib/broadsheet/molecules';

interface PinboardExchangeProps {
  data: Post;
}

export function PinboardExchange({ data: d }: PinboardExchangeProps) {
  const c = 'pinboard-offer';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text={getTagLabel(d)} prefix={c} />
      <MTitle text={d.title} prefix={c} />
      {getSourceLine(d) && (
        <div className={`${c}__detail`}>{getSourceLine(d)}</div>
      )}
      <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
      {d.status && <MStatus text={d.status.state || ''} prefix={c} extra="condensed" />}
    </div>
  );
}
