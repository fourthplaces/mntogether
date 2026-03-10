import type { ReactNode } from 'react';

export function PencilLine({ children }: { children: ReactNode }) {
  const tilt = `${(Math.random() * 2 - 1).toFixed(1)}deg`;
  return <span className="pencil-line" style={{ '--tilt': tilt } as React.CSSProperties}>{children}</span>;
}
