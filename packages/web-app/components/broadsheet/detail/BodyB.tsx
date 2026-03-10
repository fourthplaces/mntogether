import type { ReactNode } from 'react';

/** Column Narrative — left-aligned, single column, pull-quote */
export function BodyB({ children }: { children: ReactNode }) {
  return <div className="body-b">{children}</div>;
}
