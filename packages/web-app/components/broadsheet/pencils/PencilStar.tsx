import type { ReactNode } from 'react';

export function PencilStar({ children }: { children: ReactNode }) {
  const tilt = `${(Math.random() * 40 - 20).toFixed(1)}deg`;
  return <span className="pencil-star" style={{ '--tilt': tilt } as React.CSSProperties}>{children}</span>;
}
