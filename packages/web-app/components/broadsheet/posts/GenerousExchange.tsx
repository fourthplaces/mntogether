import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine, getTagLabel } from '@/lib/broadsheet/display';
import { MTitle, MMeta, MBody, MStatus } from '@/lib/broadsheet/molecules';

interface GenerousExchangeProps {
  data: Post;
}

export function GenerousExchange({ data: d }: GenerousExchangeProps) {
  const c = 'generous-offer';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div className={`${c}__header mono-sm`}>{getTagLabel(d)}</div>
      <div className={`${c}__content`}>
        <MTitle text={d.title} prefix={c} />
        <MMeta text={getSourceLine(d)} prefix={c} small />
        <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
        {d.status && <MStatus text={d.status.state || ''} prefix={c} extra="condensed" />}
      </div>
    </div>
  );
}
