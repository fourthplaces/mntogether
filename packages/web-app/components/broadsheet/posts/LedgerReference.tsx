import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTag, MTitle, MMeta, MResourceList, MBody, MUpdated } from '@/lib/broadsheet/molecules';

interface LedgerReferenceProps {
  data: Post;
}

export function LedgerReference({ data: d }: LedgerReferenceProps) {
  const c = 'led-resource';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTag text="Reference" prefix={c} />
      <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
      <MMeta text={getSourceLine(d)} prefix={c} />
      {d.items
        ? <MResourceList items={d.items} prefix={c} cols={d.cols} />
        : <MBody text={d.body || ''} prefix={c} />}
      {d.meta?.updated && <MUpdated text={d.meta.updated} prefix={c} />}
    </div>
  );
}
