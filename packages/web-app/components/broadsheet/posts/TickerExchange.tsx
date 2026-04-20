import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine, getTagLabel } from '@/lib/broadsheet/display';
import { MTag, MInlineTitle } from '@/lib/broadsheet/molecules';

interface TickerExchangeProps {
  data: Post;
}

export function TickerExchange({ data: d }: TickerExchangeProps) {
  if (d.type === 'need') {
    const c = 'tick-request';
    return (
      <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
        <MTag text={getTagLabel(d)} prefix={c} />
        <MInlineTitle text={d.title} prefix={c} />
        <span className={`${c}__meta mono-sm`}>{getSourceLine(d)}</span>
      </div>
    );
  }

  const c = 'tick-offer';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text={getTagLabel(d)} prefix={c} />
      <MInlineTitle text={d.title} prefix={c} />
      <span className={`${c}__meta mono-sm`}>{d.status?.state || getSourceLine(d)}</span>
    </div>
  );
}
