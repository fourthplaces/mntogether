"use client";

import Link from "next/link";
import { usePathname, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { useQuery } from "urql";
import { logout } from "@/lib/auth/actions";
import { PostStatsQuery } from "@/lib/graphql/posts";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface NavItem {
  href: string;
  label: string;
  icon: React.ReactNode;
  badge?: number;
  children?: NavChild[];
}

interface NavChild {
  href: string;
  label: string;
  queryParam?: string; // appended as ?postType=value
  badge?: number;
}

interface NavGroup {
  label: string;
  items: NavItem[];
}

// ---------------------------------------------------------------------------
// Icons (extracted to keep the nav array readable)
// ---------------------------------------------------------------------------

const icons = {
  dashboard: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-4 0a1 1 0 01-1-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 01-1 1" />
    </svg>
  ),
  posts: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" />
    </svg>
  ),
  workflow: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2" />
    </svg>
  ),
  editions: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v12a2 2 0 01-2 2zM3 8h18M7 8v12M17 8v12" />
    </svg>
  ),
  media: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
    </svg>
  ),
  organizations: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
    </svg>
  ),
  jobs: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
    </svg>
  ),
  tags: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A2 2 0 013 12V7a4 4 0 014-4z" />
    </svg>
  ),
  viewSite: (
    <svg className="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
    </svg>
  ),
  signOut: (
    <svg className="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
    </svg>
  ),
  collapse: (
    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
    </svg>
  ),
  chevronDown: (
    <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
    </svg>
  ),
};

// ---------------------------------------------------------------------------
// Static nav structure (badges injected at render time from query data)
// ---------------------------------------------------------------------------

const buildNavGroups = (): NavGroup[] => [
  {
    label: "Overview",
    items: [
      { href: "/admin/dashboard", label: "Dashboard", icon: icons.dashboard },
    ],
  },
  {
    label: "Content",
    items: [
      {
        href: "/admin/posts",
        label: "Posts",
        icon: icons.posts,
        children: [
          { href: "/admin/posts", label: "All Posts", queryParam: undefined },
          { href: "/admin/posts", label: "Stories", queryParam: "story" },
          { href: "/admin/posts", label: "Notices", queryParam: "notice" },
          { href: "/admin/posts", label: "Exchanges", queryParam: "exchange" },
          { href: "/admin/posts", label: "Events", queryParam: "event" },
          { href: "/admin/posts", label: "Spotlights", queryParam: "spotlight" },
          { href: "/admin/posts", label: "References", queryParam: "reference" },
        ],
      },
      { href: "/admin/workflow", label: "Workflow", icon: icons.workflow },
      { href: "/admin/editions", label: "Editions", icon: icons.editions },
      { href: "/admin/media", label: "Media", icon: icons.media },
    ],
  },
  {
    label: "Sources",
    items: [
      { href: "/admin/organizations", label: "Organizations", icon: icons.organizations },
    ],
  },
  {
    label: "System",
    items: [
      { href: "/admin/jobs", label: "Jobs", icon: icons.jobs },
      { href: "/admin/tags", label: "Tags", icon: icons.tags },
    ],
  },
];

// ---------------------------------------------------------------------------
// localStorage helpers for expanded parent items
// ---------------------------------------------------------------------------

const EXPANDED_KEY = "admin-sidebar-expanded";

function loadExpandedState(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(EXPANDED_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveExpandedState(state: Record<string, boolean>) {
  localStorage.setItem(EXPANDED_KEY, JSON.stringify(state));
}

// ---------------------------------------------------------------------------
// Badge helper — maps PostStats fields to child queryParam values
// ---------------------------------------------------------------------------

function childBadge(
  queryParam: string | undefined,
  stats: { total: number; stories: number; notices: number; exchanges: number; events: number; spotlights: number; references: number } | undefined,
): number | undefined {
  if (!stats) return undefined;
  switch (queryParam) {
    case undefined:
      return stats.total || undefined;
    case "story":
      return stats.stories || undefined;
    case "notice":
      return stats.notices || undefined;
    case "exchange":
      return stats.exchanges || undefined;
    case "event":
      return stats.events || undefined;
    case "spotlight":
      return stats.spotlights || undefined;
    case "reference":
      return stats.references || undefined;
    default:
      return undefined;
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface AdminSidebarProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
  mobileOpen: boolean;
  onMobileClose: () => void;
}

export function AdminSidebar({
  collapsed,
  onToggleCollapse,
  mobileOpen,
  onMobileClose,
}: AdminSidebarProps) {
  const pathname = usePathname();
  const searchParams = useSearchParams();

  // Expanded parent items state (for items with children)
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  useEffect(() => {
    setExpanded(loadExpandedState());
  }, []);

  const toggleExpanded = useCallback((key: string) => {
    setExpanded((prev) => {
      const next = { ...prev, [key]: !prev[key] };
      saveExpandedState(next);
      return next;
    });
  }, []);

  // Post stats for badges
  const [{ data: statsData }] = useQuery({ query: PostStatsQuery });
  const postStats = statsData?.postStats;

  // Close on Escape
  useEffect(() => {
    if (!mobileOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onMobileClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [mobileOpen, onMobileClose]);

  const navGroups = buildNavGroups();

  // Check if a nav item (or any of its children) is active
  const isItemActive = (item: NavItem) => {
    if (item.children) {
      // Parent is active if pathname matches and no child queryParam is set,
      // or if pathname starts with the href
      return pathname === item.href || pathname.startsWith(`${item.href}/`);
    }
    return pathname === item.href || pathname.startsWith(`${item.href}/`);
  };

  const isChildActive = (child: NavChild) => {
    if (!pathname.startsWith(child.href)) return false;
    const currentType = searchParams.get("postType");
    if (child.queryParam === undefined) {
      // "All Posts" is active when on /admin/posts with no postType filter
      return pathname === child.href && !currentType;
    }
    return currentType === child.queryParam;
  };

  // Render a badge pill
  const renderBadge = (count: number | undefined) => {
    if (!count) return null;
    return (
      <span className="ml-auto text-xs font-medium bg-stone-100 text-stone-500 px-1.5 py-0.5 rounded-full min-w-[1.25rem] text-center">
        {count > 999 ? "999+" : count}
      </span>
    );
  };

  const sidebarContent = (
    <div className="flex flex-col h-full bg-white border-r border-stone-200">
      {/* Logo */}
      <div className="flex items-center h-16 px-4 border-b border-stone-200 shrink-0">
        <Link
          href="/admin/dashboard"
          className="flex items-center gap-2 overflow-hidden"
          onClick={mobileOpen ? onMobileClose : undefined}
        >
          <span className="text-xl font-bold text-amber-600 shrink-0">
            {collapsed ? "MN" : "MN Together"}
          </span>
          {!collapsed && (
            <span className="text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full font-medium shrink-0">
              Admin
            </span>
          )}
        </Link>
      </div>

      {/* Nav groups */}
      <nav className="flex-1 overflow-y-auto py-4">
        {navGroups.map((group, groupIdx) => (
          <div key={group.label}>
            {groupIdx > 0 && (
              <div className="mx-3 my-3 border-t border-stone-200" />
            )}
            {!collapsed && (
              <div className="px-4 mb-1">
                <span className="text-xs font-semibold text-stone-400 uppercase tracking-wider">
                  {group.label}
                </span>
              </div>
            )}
            <div className="space-y-0.5 px-2">
              {group.items.map((item) => {
                const active = isItemActive(item);
                const hasChildren = item.children && item.children.length > 0;
                const isExpanded = hasChildren && expanded[item.href] !== false; // default expanded
                const itemBadge = item.href === "/admin/posts" ? postStats?.total : item.badge;

                if (hasChildren && !collapsed) {
                  // Parent item with expandable children
                  return (
                    <div key={item.href}>
                      <button
                        onClick={() => toggleExpanded(item.href)}
                        className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                          active
                            ? "bg-amber-50 text-amber-800"
                            : "text-stone-600 hover:bg-stone-100 hover:text-stone-900"
                        }`}
                      >
                        <span className="shrink-0">{item.icon}</span>
                        <span>{item.label}</span>
                        {renderBadge(itemBadge)}
                        <span
                          className={`ml-auto transition-transform duration-150 text-stone-400 ${
                            isExpanded ? "" : "-rotate-90"
                          }`}
                        >
                          {icons.chevronDown}
                        </span>
                      </button>

                      {/* Children */}
                      <div
                        className={`overflow-hidden transition-all duration-150 ${
                          isExpanded ? "max-h-96 opacity-100" : "max-h-0 opacity-0"
                        }`}
                      >
                        <div className="ml-5 pl-3 border-l border-stone-200 mt-0.5 space-y-0.5">
                          {item.children!.map((child) => {
                            const childActive = isChildActive(child);
                            const childHref = child.queryParam
                              ? `${child.href}?postType=${child.queryParam}`
                              : child.href;
                            const badge = childBadge(child.queryParam, postStats ?? undefined);

                            return (
                              <Link
                                key={child.queryParam ?? "all"}
                                href={childHref}
                                onClick={mobileOpen ? onMobileClose : undefined}
                                className={`flex items-center gap-2 px-3 py-1.5 rounded-md text-sm transition-colors ${
                                  childActive
                                    ? "bg-amber-100 text-amber-800 font-medium"
                                    : "text-stone-500 hover:bg-stone-50 hover:text-stone-700"
                                }`}
                              >
                                <span className="truncate">{child.label}</span>
                                {renderBadge(badge)}
                              </Link>
                            );
                          })}
                        </div>
                      </div>
                    </div>
                  );
                }

                // Regular item (no children, or sidebar is collapsed)
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    onClick={mobileOpen ? onMobileClose : undefined}
                    title={collapsed ? item.label : undefined}
                    className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      collapsed ? "justify-center" : ""
                    } ${
                      active
                        ? "bg-amber-100 text-amber-800"
                        : "text-stone-600 hover:bg-stone-100 hover:text-stone-900"
                    }`}
                  >
                    <span className="shrink-0">{item.icon}</span>
                    {!collapsed && (
                      <>
                        <span>{item.label}</span>
                        {renderBadge(itemBadge)}
                      </>
                    )}
                  </Link>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      {/* Bottom section */}
      <div className="border-t border-stone-200 p-2 shrink-0 space-y-0.5">
        <Link
          href="/"
          onClick={mobileOpen ? onMobileClose : undefined}
          title={collapsed ? "View Site" : undefined}
          className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900 transition-colors ${
            collapsed ? "justify-center" : ""
          }`}
        >
          {icons.viewSite}
          {!collapsed && <span>View Site</span>}
        </Link>
        <form action={logout}>
          <button
            type="submit"
            title={collapsed ? "Sign Out" : undefined}
            className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900 transition-colors ${
              collapsed ? "justify-center" : ""
            }`}
          >
            {icons.signOut}
            {!collapsed && <span>Sign Out</span>}
          </button>
        </form>
      </div>

      {/* Collapse toggle (desktop only) */}
      <div className="hidden lg:flex border-t border-stone-200 p-2 shrink-0">
        <button
          onClick={onToggleCollapse}
          className="w-full flex items-center justify-center p-2 rounded-lg text-stone-400 hover:bg-stone-100 hover:text-stone-600 transition-colors"
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          <span className={`transition-transform ${collapsed ? "rotate-180" : ""}`}>
            {icons.collapse}
          </span>
        </button>
      </div>
    </div>
  );

  return (
    <>
      {/* Desktop sidebar */}
      <aside
        className={`hidden lg:block shrink-0 h-screen sticky top-0 transition-[width] duration-200 ${
          collapsed ? "w-16" : "w-60"
        }`}
      >
        {sidebarContent}
      </aside>

      {/* Mobile overlay */}
      {mobileOpen && (
        <>
          <div
            className="fixed inset-0 bg-black/50 z-40 lg:hidden"
            onClick={onMobileClose}
          />
          <aside className="fixed inset-y-0 left-0 w-60 z-50 lg:hidden shadow-xl animate-slide-in">
            {sidebarContent}
          </aside>
        </>
      )}
    </>
  );
}
