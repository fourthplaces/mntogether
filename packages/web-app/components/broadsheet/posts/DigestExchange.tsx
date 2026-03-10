import type { Post } from '@/lib/broadsheet/types';
import { getTagLabel, getContactDisplay } from '@/lib/broadsheet/display';
import { MTitle, MBody } from '@/lib/broadsheet/molecules';

interface DigestExchangeProps {
  data: Post;
}

export function DigestExchange({ data: d }: DigestExchangeProps) {
  const c = 'dig-request';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div className={`${c}__cat mono-sm`}>{getTagLabel(d)}</div>
      <MTitle text={d.title} prefix={c} extra="condensed" />
      <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
      {getContactDisplay(d) && (
        <span className={`${c}__contact mono-sm`} dangerouslySetInnerHTML={{ __html: getContactDisplay(d) }} />
      )}
    </div>
  );
}
