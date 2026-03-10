import type { ReactNode } from 'react';

type CellSpan = 1 | 2 | 3 | 4 | 6;

interface CellProps {
  span: CellSpan;
  children: ReactNode;
}

export function Cell({ span, children }: CellProps) {
  return (
    <div className={`cell cell--span-${span}`} data-debug={`Cell.span-${span}`}>
      {children}
    </div>
  );
}
