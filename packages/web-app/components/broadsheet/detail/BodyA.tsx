import type { ReactNode } from 'react';

/** Broadsheet Editorial — justified, drop cap, blockquotes */
export function BodyA({ children }: { children: ReactNode }) {
  return <div className="body-a">{children}</div>;
}
