"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { useQuery } from "urql";
import { logout } from "@/lib/auth/actions";
import { LatestEditionsQuery } from "@/lib/graphql/editions";
import { Separator } from "@/components/ui/separator";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  LayoutDashboard,
  Columns3,
  Map,
  FileText,
  Image,
  Building2,
  RefreshCw,
  Tag,
  Megaphone,
  Radio,
  PenSquare,
  ExternalLink,
  LogOut,
  ChevronsLeft,
} from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface NavItem {
  href: string;
  label: string;
  icon: React.ReactNode;
  badge?: number;
  labelSuffix?: string;
}

interface NavGroup {
  label: string;
  items: NavItem[];
}

// ---------------------------------------------------------------------------
// Static nav structure (badges injected at render time from query data)
// ---------------------------------------------------------------------------

const buildNavGroups = (): NavGroup[] => [
  {
    label: "Overview",
    items: [
      { href: "/admin/dashboard", label: "Dashboard", icon: <LayoutDashboard className="w-5 h-5" /> },
    ],
  },
  {
    label: "Broadsheet",
    items: [
      { href: "/admin/editions", label: "Counties", icon: <Map className="w-5 h-5" /> },
      { href: "/admin/workflow", label: "Review Board", icon: <Columns3 className="w-5 h-5" />, labelSuffix: "Beta" },
    ],
  },
  {
    label: "Content",
    items: [
      { href: "/admin/signal", label: "Signal", icon: <Radio className="w-5 h-5" /> },
      { href: "/admin/posts", label: "Editorial", icon: <PenSquare className="w-5 h-5" /> },
      { href: "/admin/notes", label: "Notes", icon: <Megaphone className="w-5 h-5" /> },
      { href: "/admin/media", label: "Media", icon: <Image className="w-5 h-5" /> },
    ],
  },
  {
    label: "Sources",
    items: [
      { href: "/admin/organizations", label: "Sources", icon: <Building2 className="w-5 h-5" /> },
    ],
  },
  {
    label: "System",
    items: [
      { href: "/admin/tags", label: "Tags", icon: <Tag className="w-5 h-5" /> },
    ],
  },
];

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

  // Derive review badge from latest editions (count of draft + in_review)
  const [{ data: editionsData }] = useQuery({ query: LatestEditionsQuery });
  const reviewBadge = editionsData?.latestEditions
    ? editionsData.latestEditions.filter(
        (e) => e.status === "draft" || e.status === "in_review"
      ).length || undefined
    : undefined;

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

  // Check if a nav item is active
  const isItemActive = (item: NavItem) => {
    return pathname === item.href || pathname.startsWith(`${item.href}/`);
  };

  // Render a badge pill
  const renderBadge = (count: number | undefined) => {
    if (!count) return null;
    return (
      <span className="ml-auto text-xs font-medium bg-muted text-muted-foreground px-1.5 py-0.5 rounded-full min-w-[1.25rem] text-center">
        {count > 999 ? "999+" : count}
      </span>
    );
  };

  // Wrap content in tooltip when sidebar is collapsed
  const MaybeTooltip = ({ label, children }: { label: string; children: React.ReactNode }) => {
    if (!collapsed) return <>{children}</>;
    return (
      <Tooltip>
        <TooltipTrigger render={<span />}>{children}</TooltipTrigger>
        <TooltipContent side="right">{label}</TooltipContent>
      </Tooltip>
    );
  };

  const sidebarContent = (
    <div className="flex flex-col h-full bg-card border-r border-border">
      {/* Logo */}
      <div className="flex items-center h-16 px-4 border-b border-border shrink-0">
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
      <ScrollArea className="flex-1 py-4">
        {navGroups.map((group, groupIdx) => (
          <div key={group.label}>
            {groupIdx > 0 && (
              <Separator className="mx-3 my-3" />
            )}
            {!collapsed && (
              <div className="px-4 mb-1">
                <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                  {group.label}
                </span>
              </div>
            )}
            <div className="space-y-0.5 px-2">
              {group.items.map((item) => {
                const active = isItemActive(item);
                const itemBadge =
                  item.href === "/admin/workflow" ? reviewBadge : item.badge;

                return (
                  <MaybeTooltip key={item.href} label={item.label}>
                    <Link
                      href={item.href}
                      onClick={mobileOpen ? onMobileClose : undefined}
                      className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                        collapsed ? "justify-center" : ""
                      } ${
                        active
                          ? "bg-amber-100 text-amber-800"
                          : "text-muted-foreground hover:bg-muted hover:text-foreground"
                      }`}
                    >
                      <span className="shrink-0">{item.icon}</span>
                      {!collapsed && (
                        <>
                          <span>{item.label}</span>
                          {item.labelSuffix && (
                            <span className="text-[10px] font-medium bg-muted text-muted-foreground px-1.5 py-0.5 rounded-full uppercase tracking-wide">
                              {item.labelSuffix}
                            </span>
                          )}
                          {renderBadge(itemBadge)}
                        </>
                      )}
                    </Link>
                  </MaybeTooltip>
                );
              })}
            </div>
          </div>
        ))}
      </ScrollArea>

      {/* Bottom section */}
      <div className="border-t border-border p-2 shrink-0 space-y-0.5">
        <MaybeTooltip label="View Site">
          <Link
            href="/"
            onClick={mobileOpen ? onMobileClose : undefined}
            className={`flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors ${
              collapsed ? "justify-center" : ""
            }`}
          >
            <ExternalLink className="w-5 h-5 shrink-0" />
            {!collapsed && <span>View Site</span>}
          </Link>
        </MaybeTooltip>
        <form action={logout}>
          <MaybeTooltip label="Sign Out">
            <button
              type="submit"
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors ${
                collapsed ? "justify-center" : ""
              }`}
            >
              <LogOut className="w-5 h-5 shrink-0" />
              {!collapsed && <span>Sign Out</span>}
            </button>
          </MaybeTooltip>
        </form>
      </div>

      {/* Collapse toggle (desktop only) */}
      <div className="hidden lg:flex border-t border-border p-2 shrink-0">
        <Tooltip>
          <TooltipTrigger render={<button
              onClick={onToggleCollapse}
              className="w-full flex items-center justify-center p-2 rounded-lg text-muted-foreground hover:bg-muted hover:text-foreground transition-colors"
            />}>
              <span className={`transition-transform ${collapsed ? "rotate-180" : ""}`}>
                <ChevronsLeft className="w-5 h-5" />
              </span>
          </TooltipTrigger>
          <TooltipContent side="right">
            {collapsed ? "Expand sidebar" : "Collapse sidebar"}
          </TooltipContent>
        </Tooltip>
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
