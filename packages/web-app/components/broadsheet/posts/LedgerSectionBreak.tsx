import type { Post } from '@/lib/broadsheet/types';

interface LedgerSectionBreakProps {
  data: Post;
}

export function LedgerSectionBreak({ data: d }: LedgerSectionBreakProps) {
  return (
    <div className="led-section-break" data-debug="Post.led-section-break">
      <div className="led-section-break__title" dangerouslySetInnerHTML={{ __html: d.title }} />
      {d.sub && <div className="led-section-break__sub">{d.sub}</div>}
    </div>
  );
}
