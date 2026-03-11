"use client";

import { Suspense } from "react";
import { AdminSidebar } from "@/components/admin/AdminSidebar";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { GraphQLErrorBoundary } from "@/components/admin/GraphQLErrorBoundary";

export default function AdminAppLayout({ children }: { children: React.ReactNode }) {
  return (
    <SidebarProvider>
      <Suspense>
        <AdminSidebar />
      </Suspense>
      <SidebarInset>
        <header className="flex h-14 shrink-0 items-center gap-2 border-b border-border px-4 md:hidden">
          <SidebarTrigger className="-ml-1" />
          <Separator orientation="vertical" className="mr-2 h-4" />
          <span className="text-lg font-bold">Root Editorial</span>
        </header>
        <main className="flex-1 overflow-y-auto">
          <GraphQLErrorBoundary>{children}</GraphQLErrorBoundary>
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}
