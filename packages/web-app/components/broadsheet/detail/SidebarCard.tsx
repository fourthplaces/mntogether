import type { ReactNode } from 'react';

interface SidebarCardProps {
  header: string;
  children: ReactNode;
}

export function SidebarCard({ header, children }: SidebarCardProps) {
  return (
    <div className="sidebar-card">
      <div className="sidebar-card__header mono-sm">{header}</div>
      {children}
    </div>
  );
}
