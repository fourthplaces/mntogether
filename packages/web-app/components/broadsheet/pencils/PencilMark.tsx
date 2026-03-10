import type { ReactNode } from 'react';

export function PencilMark({ children }: { children: ReactNode }) {
  return <span className="pencil-mark">{children}</span>;
}
