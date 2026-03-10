import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine, getTagLabel } from '@/lib/broadsheet/display';
import { MTitle, MBody, MMeta } from '@/lib/broadsheet/molecules';

interface AlertNoticeProps {
  data: Post;
}

export function AlertNotice({ data: d }: AlertNoticeProps) {
  const c = 'alert-urgent';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div className={`${c}__flag mono-sm`}>{getTagLabel(d)}</div>
      <MTitle text={d.title} prefix={c} />
      <MBody text={d.body} prefix={c} clamp={d.clamp || 3} />
      <MMeta text={getMetaLine(d)} prefix={c} small />
    </div>
  );
}
