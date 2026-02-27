"use client";

import { Suspense, useState, useEffect } from "react";
import { AdminSidebar } from "@/components/admin/AdminSidebar";
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
    <div className="min-h-screen bg-stone-50 flex">
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
        <header className="lg:hidden flex items-center h-14 px-4 bg-white border-b border-stone-200 shrink-0">
          <button
            onClick={() => setMobileOpen(true)}
            className="p-2 -ml-2 rounded-lg text-stone-600 hover:bg-stone-100"
            aria-label="Open menu"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
          <span className="ml-3 text-lg font-bold text-amber-600">MN Together</span>
          <span className="ml-2 text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full font-medium">
            Admin
          </span>
        </header>

        <main className="flex-1 overflow-y-auto pb-20">
          <GraphQLErrorBoundary>{children}</GraphQLErrorBoundary>
        </main>
      </div>
    </div>
  );
}
