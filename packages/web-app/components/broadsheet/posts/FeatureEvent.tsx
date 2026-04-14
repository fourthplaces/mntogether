import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTitle, MMeta, MWhen } from '@/lib/broadsheet/molecules';

interface FeatureEventProps {
  data: Post;
}

export function FeatureEvent({ data: d }: FeatureEventProps) {
  const c = 'feat-event';

  const dayEl = (
    <div className={`${c}__day condensed`}>{d.day || ''}</div>
  );

  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <div className={`${c}__month mono-md`}>{d.month || ''}</div>
      <div
        className="pencil-circle"
        data-label={d.circleLabel || ''}
        style={{ '--tilt': `${(Math.random() * -8 - 2).toFixed(1)}deg` } as React.CSSProperties}
      >
        {dayEl}
      </div>
      <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
      <MMeta text={getSourceLine(d)} prefix={c} small />
      {d.when && <MWhen text={d.when} prefix={c} />}
    </div>
  );
}
