import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine, getSourceLine, getTagLabel } from '@/lib/broadsheet/display';
import { MTag, MTime } from '@/lib/broadsheet/molecules';

interface TickerNoticeProps {
  data: Post;
}

export function TickerNotice({ data: d }: TickerNoticeProps) {
  if (d.tags?.includes('urgent')) {
    const c = 'tick-urgent';
    return (
      <div className={c} data-debug={`Post.${c}`}>
        <MTag text={getTagLabel(d)} prefix={c} />
        <span className={`${c}__title`} dangerouslySetInnerHTML={{ __html: d.title }} />
        <span className={`${c}__meta mono-sm`}>{getMetaLine(d)}</span>
      </div>
    );
  }

  const c = 'tick-update';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      {d.meta?.timestamp && <MTime text={d.meta.timestamp} prefix={c} />}
      <span className={`${c}__title`} dangerouslySetInnerHTML={{ __html: d.title }} />
      <span className={`${c}__meta mono-sm`}>{getSourceLine(d)}</span>
    </div>
  );
}
