import type { ReactNode } from 'react';
import type { TitleSize } from '@/lib/broadsheet/detail-types';

interface TitleAProps {
  children: ReactNode;
  size?: TitleSize;
  deck?: string;
}

export function TitleA({ children, size = 'story', deck }: TitleAProps) {
  return (
    <>
      <h1 className={`title-a title-a--${size}`}>{children}</h1>
      {deck && <div className="title-a__deck">{deck}</div>}
    </>
  );
}

interface TitleBProps {
  children: ReactNode;
  size?: TitleSize;
  summary?: string;
}

export function TitleB({ children, size = 'story', summary }: TitleBProps) {
  return (
    <>
      <h1 className={`title-b title-b--${size}`}>{children}</h1>
      {summary && <div className="title-b__summary">{summary}</div>}
    </>
  );
}
