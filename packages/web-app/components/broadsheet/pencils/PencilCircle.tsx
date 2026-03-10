import type { ReactNode } from 'react';

interface PencilCircleProps {
  label: string;
  children: ReactNode;
}

export function PencilCircle({ label, children }: PencilCircleProps) {
  const tilt = `${(Math.random() * -8 - 2).toFixed(1)}deg`;
  return (
    <div className="pencil-circle" data-label={label} style={{ '--tilt': tilt } as React.CSSProperties}>
      {children}
    </div>
  );
}
