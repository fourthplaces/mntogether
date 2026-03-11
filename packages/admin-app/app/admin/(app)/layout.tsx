"use client";

import { Suspense, useState, useEffect } from "react";
import { Menu } from "lucide-react";
import { AdminSidebar } from "@/components/admin/AdminSidebar";
import { Button } from "@/components/ui/button";
import { GraphQLErrorBoundary } from "@/components/admin/GraphQLErrorBoundary";

const COLLAPSED_KEY = "admin-sidebar-collapsed";

export default function AdminAppLayout({ children }: { children: React.ReactNode }) {
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  // Restore collapsed state from localStorage
  useEffect(() => {
    const stored = localStorage.getItem(COLLAPSED_KEY);
    if (stored === "true") setCollapsed(true);
  }, []);

  const handleToggleCollapse = () => {
    setCollapsed((prev) => {
      const next = !prev;
      localStorage.setItem(COLLAPSED_KEY, String(next));
      return next;
    });
  };

  return (
    <div className="h-screen bg-background flex overflow-hidden">
      <Suspense>
        <AdminSidebar
          collapsed={collapsed}
          onToggleCollapse={handleToggleCollapse}
          mobileOpen={mobileOpen}
          onMobileClose={() => setMobileOpen(false)}
        />
      </Suspense>

      <div className="flex-1 flex flex-col min-w-0">
        {/* Mobile top bar */}
        <header className="lg:hidden flex items-center h-14 px-4 bg-card border-b border-border shrink-0">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setMobileOpen(true)}
            className="-ml-2 text-muted-foreground"
            aria-label="Open menu"
          >
            <Menu className="w-6 h-6" />
          </Button>
          <span className="ml-3 text-lg font-bold text-amber-600">MN Together</span>
          <span className="ml-2 text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full font-medium">
            Admin
          </span>
        </header>

        <main className="flex-1 overflow-y-auto">
          <GraphQLErrorBoundary>{children}</GraphQLErrorBoundary>
        </main>
      </div>
    </div>
  );
}
