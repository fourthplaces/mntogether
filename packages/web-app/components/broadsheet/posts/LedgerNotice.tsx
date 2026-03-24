import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine, getSourceLine, getTagLabel } from '@/lib/broadsheet/display';
import { MTag, MTitle, MMeta, MBody, MCtaLink, MReadMore, MTime } from '@/lib/broadsheet/molecules';

interface LedgerNoticeProps {
  data: Post;
}

export function LedgerNotice({ data: d }: LedgerNoticeProps) {
  const isAction = d.tags?.includes('action');
  const isUrgent = d.tags?.includes('urgent');

  if (isAction) {
    const c = 'led-cta';
    return (
      <div className={c} data-debug={`Post.${c}`}>
        <MTag text="Action" prefix={c} />
        <MTitle text={d.title} prefix={c} />
        <MMeta text={getMetaLine(d)} prefix={c} />
        <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
        {d.link && <MCtaLink href={d.link.url} text={d.link.label || ''} prefix={c} />}
      </div>
    );
  }

  if (isUrgent) {
    const c = 'led-urgent';
    return (
      <div className={c} data-debug={`Post.${c}`}>
        <MTag text={getTagLabel(d)} prefix={c} />
        <MTitle text={d.title} prefix={c} />
        <MMeta text={getMetaLine(d)} prefix={c} />
        <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
        {d.readMore && <MReadMore href={d.readMore} />}
      </div>
    );
  }

  // Default update treatment
  const c = 'led-update';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      {d.date && <MTime text={d.date} prefix={c} />}
      <MTitle text={d.title} prefix={c} />
      <MMeta text={getSourceLine(d)} prefix={c} />
      <MBody text={d.body} prefix={c} clamp={d.clamp || 3} />
    </div>
  );
}
