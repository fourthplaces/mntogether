import type { ReactNode } from 'react';

type RowVariant = 'lead' | 'lead-stack' | 'pair' | 'pair-stack' | 'trio' | 'trio-mixed' | 'full';

interface RowProps {
  variant: RowVariant;
  rule?: boolean;
  children: ReactNode;
}

export function Row({ variant, rule, children }: RowProps) {
  return (
    <div className={`row row--${variant}${rule ? ' row--rule' : ''}`} data-debug={`Row.${variant}`}>
      {children}
    </div>
  );
}
