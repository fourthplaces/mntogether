import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine, getTagLabel, getContactDisplay } from '@/lib/broadsheet/display';
import { MTag, MTitle, MMeta, MBody, MContact, MStatus } from '@/lib/broadsheet/molecules';

interface LedgerExchangeProps {
  data: Post;
}

export function LedgerExchange({ data: d }: LedgerExchangeProps) {
  const isNeed = d.type === 'need';

  if (isNeed) {
    const c = 'led-request';
    return (
      <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
        <MTag text={getTagLabel(d)} prefix={c} />
        <MTitle text={d.title} prefix={c} />
        <MMeta text={getSourceLine(d)} prefix={c} />
        <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
        {getContactDisplay(d) && <MContact text={getContactDisplay(d)} prefix={c} />}
      </div>
    );
  }

  // Aid/offer treatment
  const c = 'led-offer';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text={getTagLabel(d)} prefix={c} />
      <MTitle text={d.title} prefix={c} />
      <MMeta text={getSourceLine(d)} prefix={c} />
      <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
      {d.status && <MStatus text={d.status.state || ''} prefix={c} />}
    </div>
  );
}
