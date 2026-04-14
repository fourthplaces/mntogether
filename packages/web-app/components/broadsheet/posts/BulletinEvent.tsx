import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTag, MTitle, MMeta, MBody, MWhen } from '@/lib/broadsheet/molecules';

interface BulletinEventProps {
  data: Post;
}

/** Bulletin event has a unique calendar widget (bul-event__cal) */
export function BulletinEvent({ data: d }: BulletinEventProps) {
  const c = 'bul-event';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <div className={`${c}__cal`}>
        <span className={`${c}__month mono-sm`}>{d.month || ''}</span>
        <span className={`${c}__day`}>{d.day || ''}</span>
      </div>
      <div className={`${c}__info`}>
        <MTag text="Event" prefix={c} />
        <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
        <MMeta text={getSourceLine(d)} prefix={c} />
        <MBody text={d.body} prefix={c} clamp={d.clamp ?? 3} />
        {d.when && <MWhen text={d.when} prefix={c} />}
      </div>
    </div>
  );
}
