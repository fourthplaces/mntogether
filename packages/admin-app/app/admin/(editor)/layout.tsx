"use client";

import { GraphQLErrorBoundary } from "@/components/admin/GraphQLErrorBoundary";

export default function EditorLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="h-screen flex flex-col bg-surface overflow-hidden">
      <GraphQLErrorBoundary>{children}</GraphQLErrorBoundary>
    </div>
  );
}
