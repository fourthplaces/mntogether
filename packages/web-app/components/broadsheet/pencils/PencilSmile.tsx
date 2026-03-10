import type { ReactNode } from 'react';

export function PencilSmile({ children }: { children: ReactNode }) {
  const tilt = `${(Math.random() * 30).toFixed(1)}deg`;
  return <span className="pencil-smile" style={{ '--tilt': tilt } as React.CSSProperties}>{children}</span>;
}
