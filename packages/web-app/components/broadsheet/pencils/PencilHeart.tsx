import type { ReactNode } from 'react';

export function PencilHeart({ children }: { children: ReactNode }) {
  const tilt = `${(Math.random() * 40 - 20).toFixed(1)}deg`;
  return <span className="pencil-heart" style={{ '--tilt': tilt } as React.CSSProperties}>{children}</span>;
}
