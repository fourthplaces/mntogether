import type { Post } from '@/lib/broadsheet/types';
import { getTagLabel } from '@/lib/broadsheet/display';
import { MTitle, MBody, MCtaLink } from '@/lib/broadsheet/molecules';

interface FeatureNoticeProps {
  data: Post;
}

export function FeatureNotice({ data: d }: FeatureNoticeProps) {
  if (d.type === 'action') {
    const c = 'feat-cta';
    return (
      <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
        <MTitle text={d.title} prefix={c} extra="condensed" pencilMark={d.pencilMark} />
        <MBody text={d.body} prefix={c} clamp={d.clamp || 3} />
        {d.link && <MCtaLink href={d.link.url} text={d.link.label || ''} prefix={c} />}
      </div>
    );
  }

  // Urgent/alert treatment
  const c = 'feat-urgent';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <div className={`${c}__kicker mono-md`}>{getTagLabel(d)}</div>
      <MTitle text={d.title} prefix={c} extra="condensed" pencilMark={d.pencilMark} />
      <MBody text={d.body} prefix={c} />
      {d.readMore && (
        <a href={d.readMore} className={`${c}__action mono-md`}>Read more &rarr;</a>
      )}
    </div>
  );
}
