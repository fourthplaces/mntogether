import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTitle } from '@/lib/broadsheet/molecules';

interface DigestNoticeProps {
  data: Post;
}

export function DigestNotice({ data: d }: DigestNoticeProps) {
  const c = 'dig-update';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div className={`${c}__date condensed`}>{d.date || ''}</div>
      <div className={`${c}__content`}>
        <MTitle text={d.title} prefix={c} />
        <div className={`${c}__source mono-sm`}>{getSourceLine(d)}</div>
      </div>
    </div>
  );
}
