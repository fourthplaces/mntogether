import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTitle, MMeta, MResourceList, MBody } from '@/lib/broadsheet/molecules';

interface QuickRefProps {
  data: Post;
}

export function QuickRef({ data: d }: QuickRefProps) {
  const c = 'quickref-resource';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      {d.count && <div className={`${c}__count condensed`}>{d.count}</div>}
      <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
      <MMeta text={getSourceLine(d)} prefix={c} small />
      {d.items
        ? <MResourceList items={d.items} prefix={c} />
        : <MBody text={d.body || ''} prefix={c} />}
    </div>
  );
}
