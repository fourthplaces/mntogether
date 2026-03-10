import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTag, MTitle, MMeta, MBody, MWhen } from '@/lib/broadsheet/molecules';

interface GazetteEventProps {
  data: Post;
}

export function GazetteEvent({ data: d }: GazetteEventProps) {
  const c = 'gaz-event';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div className={`${c}__row`}>
        <MTag text="Event" prefix={c} />
        <span className={`${c}__date mono-md`}>{d.day || ''} {d.month || ''}</span>
      </div>
      <MTitle text={d.title} prefix={c} />
      <MMeta text={getSourceLine(d)} prefix={c} />
      <MBody text={d.body} prefix={c} clamp={d.clamp || 3} />
      {d.when && <MWhen text={d.when} prefix={c} />}
    </div>
  );
}
