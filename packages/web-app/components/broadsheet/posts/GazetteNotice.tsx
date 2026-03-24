import type { Post } from '@/lib/broadsheet/types';
import { getMetaLine, getSourceLine, getTagLabel } from '@/lib/broadsheet/display';
import { MTag, MTitle, MMeta, MBody, MCtaLink, MReadMore, MTime } from '@/lib/broadsheet/molecules';

interface GazetteNoticeProps {
  data: Post;
}

export function GazetteNotice({ data: d }: GazetteNoticeProps) {
  const isAction = d.tags?.includes('action');
  const isUrgent = d.tags?.includes('urgent');

  if (isAction) {
    const c = 'gaz-cta';
    return (
      <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
        <MTag text="Action" prefix={c} />
        <MTitle text={d.title} prefix={c} />
        <MMeta text={getMetaLine(d)} prefix={c} />
        <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
        {d.link && <MCtaLink href={d.link.url} text={d.link.label || ''} prefix={c} />}
      </div>
    );
  }

  if (isUrgent) {
    const c = 'gaz-urgent';
    return (
      <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
        <MTag text={getTagLabel(d)} prefix={c} />
        <MTitle text={d.title} prefix={c} />
        <MMeta text={getMetaLine(d)} prefix={c} />
        <MBody text={d.body} prefix={c} clamp={d.clamp || 4} />
        {d.readMore && <MReadMore href={d.readMore} />}
      </div>
    );
  }

  // Default update treatment
  const c = 'gaz-update';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      {d.date && <MTime text={d.date} prefix={c} />}
      <MTitle text={d.title} prefix={c} />
      <MMeta text={getSourceLine(d)} prefix={c} />
      <MBody text={d.body} prefix={c} clamp={d.clamp || 3} />
    </div>
  );
}
