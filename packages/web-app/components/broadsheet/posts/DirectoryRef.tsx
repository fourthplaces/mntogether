import type { Post } from '@/lib/broadsheet/types';
import { getSourceLine } from '@/lib/broadsheet/display';
import { MTitle, MMeta, MResourceList, MBody, MUpdated } from '@/lib/broadsheet/molecules';

interface DirectoryRefProps {
  data: Post;
}

export function DirectoryRef({ data: d }: DirectoryRefProps) {
  const c = 'directory-resource';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <MTitle text={d.title} prefix={c} extra="condensed" pencilMark={d.pencilMark} />
      <MMeta text={getSourceLine(d)} prefix={c} small />
      {d.items
        ? <MResourceList items={d.items} prefix={c} cols={d.cols} />
        : <MBody text={d.body || ''} prefix={c} />}
      {d.meta?.updated && <MUpdated text={d.meta.updated} prefix={c} />}
    </div>
  );
}
