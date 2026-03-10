import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';

interface BroadsheetTickerNoticeProps {
  data: Post;
}

export function BroadsheetTickerNotice({ data: d }: BroadsheetTickerNoticeProps) {
  const c = 'ticker-update';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <span className={`${c}__time mono-sm`}>{d.meta?.timestamp || ''}</span>
      <span className={`${c}__title condensed`} dangerouslySetInnerHTML={{ __html: d.title }} />
      <span className={`${c}__source mono-sm`}>{getSourceLine(d)}</span>
    </div>
  );
}
