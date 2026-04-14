import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTitle, MMeta, MBody, MWhen } from '@/lib/broadsheet/molecules';

interface CardEventProps {
  data: Post;
}

export function CardEvent({ data: d }: CardEventProps) {
  const c = 'card-event';

  const dateBlock = (
    <>
      <div className={`${c}__date condensed`}>{d.month || ''} {d.day || ''}</div>
      {d.dow && <div className={`${c}__dow mono-sm`}>{d.dow}</div>}
    </>
  );

  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <div className={`${c}__header`}>
        {d.circleLabel ? (
          <div
            className="pencil-circle"
            data-label={d.circleLabel}
            style={{ '--tilt': `${(Math.random() * -8 - 2).toFixed(1)}deg` } as React.CSSProperties}
          >
            {dateBlock}
          </div>
        ) : dateBlock}
      </div>
      <div className={`${c}__content`}>
        <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
        <MMeta text={getSourceLine(d)} prefix={c} small />
        <MBody text={d.body} prefix={c} clamp={d.clamp ?? 3} />
        {d.when && <MWhen text={d.when} prefix={c} md />}
      </div>
    </div>
  );
}
