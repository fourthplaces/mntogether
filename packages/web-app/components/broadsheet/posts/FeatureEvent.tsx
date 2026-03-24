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
      {d.circleLabel ? (
        <div
          className="pencil-circle"
          data-label={d.circleLabel}
          style={{ '--tilt': `${(Math.random() * -8 - 2).toFixed(1)}deg` } as React.CSSProperties}
        >
          {dayEl}
        </div>
      ) : dayEl}
      <MTitle text={d.title} prefix={c} />
      <MMeta text={getSourceLine(d)} prefix={c} small />
      {d.when && <MWhen text={d.when} prefix={c} />}
    </div>
  );
}
