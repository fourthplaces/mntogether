import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { usePostDetailLink } from '@/lib/broadsheet/post-link-context';

interface BroadsheetTickerNoticeProps {
  data: Post;
}

// This template uses a custom title class (`condensed` modifier), so it
// needs its own link wiring rather than using MInlineTitle.
export function BroadsheetTickerNotice({ data: d }: BroadsheetTickerNoticeProps) {
  const c = 'ticker-update';
  const href = usePostDetailLink();
  const titleClass = `${c}__title condensed${href ? ' post-title-link' : ''}`;
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <span className={`${c}__time mono-sm`}>{d.date || ''}</span>
      {href ? (
        <a href={href} className={titleClass} dangerouslySetInnerHTML={{ __html: d.title }} />
      ) : (
        <span className={titleClass} dangerouslySetInnerHTML={{ __html: d.title }} />
      )}
      <span className={`${c}__source mono-sm`}>{getSourceLine(d)}</span>
    </div>
  );
}
