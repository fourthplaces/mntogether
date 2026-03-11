"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useQuery } from "urql";
import { logout } from "@/lib/auth/actions";
import {
  LayoutDashboard,
  Map,
  Columns3,
  Image,
  Building2,
  Tag,
  Radio,
  PenSquare,
  Megaphone,
  ExternalLink,
  LogOut,
  Settings,
  Users,
  ChevronsUpDown,
  UserCircle,
} from "lucide-react";
import { LatestEditionsQuery } from "@/lib/graphql/editions";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

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
// Static nav structure
// ---------------------------------------------------------------------------

const navGroups: NavGroup[] = [
  {
    label: "Overview",
    items: [
      { href: "/admin/dashboard", label: "Dashboard", icon: <LayoutDashboard /> },
    ],
  },
  {
    label: "Broadsheet",
    items: [
      { href: "/admin/editions", label: "Counties", icon: <Map /> },
      { href: "/admin/workflow", label: "Review Board", icon: <Columns3 />, labelSuffix: "Beta" },
    ],
  },
  {
    label: "Content",
    items: [
      { href: "/admin/signal", label: "Signal", icon: <Radio /> },
      { href: "/admin/posts", label: "Editorial", icon: <PenSquare /> },
      { href: "/admin/notes", label: "Notes", icon: <Megaphone /> },
      { href: "/admin/media", label: "Media", icon: <Image /> },
    ],
  },
  {
    label: "Sources",
    items: [
      { href: "/admin/organizations", label: "Sources", icon: <Building2 /> },
    ],
  },
  {
    label: "System",
    items: [
      { href: "/admin/tags", label: "Tags", icon: <Tag /> },
      { href: "/admin/users", label: "Users", icon: <Users /> },
    ],
  },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AdminSidebar() {
  const pathname = usePathname();

  // Derive review badge from latest editions (count of draft + in_review)
  const [{ data: editionsData }] = useQuery({ query: LatestEditionsQuery });
  const reviewBadge = editionsData?.latestEditions
    ? editionsData.latestEditions.filter(
        (e) => e.status === "draft" || e.status === "in_review"
      ).length || undefined
    : undefined;

  const isItemActive = (item: NavItem) =>
    pathname === item.href || pathname.startsWith(`${item.href}/`);

  return (
    <Sidebar collapsible="icon">
      {/* Header / Logo */}
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" render={<Link href="/admin/dashboard" />}>
              <span className="text-xl font-bold text-amber-600">RE</span>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-bold">Root Editorial</span>
                <span className="truncate text-xs text-muted-foreground">mntogether.org</span>
              </div>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      {/* Nav groups */}
      <SidebarContent>
        {navGroups.map((group) => (
          <SidebarGroup key={group.label}>
            <SidebarGroupLabel>{group.label}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {group.items.map((item) => {
                  const active = isItemActive(item);
                  const itemBadge =
                    item.href === "/admin/workflow" ? reviewBadge : item.badge;

                  return (
                    <SidebarMenuItem key={item.href}>
                      <SidebarMenuButton
                        render={<Link href={item.href} />}
                        tooltip={item.label}
                        isActive={active}
                      >
                        {item.icon}
                        <span>{item.label}</span>
                        {item.labelSuffix && (
                          <span className="text-[10px] font-medium bg-muted text-muted-foreground px-1.5 py-0.5 rounded-full uppercase tracking-wide">
                            {item.labelSuffix}
                          </span>
                        )}
                      </SidebarMenuButton>
                      {itemBadge ? (
                        <SidebarMenuBadge>{itemBadge > 999 ? "999+" : itemBadge}</SidebarMenuBadge>
                      ) : null}
                    </SidebarMenuItem>
                  );
                })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        ))}

        {/* View site link at bottom of content */}
        <SidebarGroup className="mt-auto">
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton render={<Link href="/" />} tooltip="View Site">
                  <ExternalLink />
                  <span>View Site</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      {/* Footer: account dropdown */}
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <SidebarMenuButton
                    size="lg"
                    className="data-popup-open:bg-sidebar-accent data-popup-open:text-sidebar-accent-foreground"
                  />
                }
              >
                <UserCircle className="size-5" />
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-medium">Account</span>
                </div>
                <ChevronsUpDown className="ml-auto size-4" />
              </DropdownMenuTrigger>
              <DropdownMenuContent
                side="top"
                align="start"
                sideOffset={4}
                className="w-(--sidebar-width) min-w-56"
              >
                <DropdownMenuGroup>
                  <DropdownMenuItem render={<Link href="/admin/account" />}>
                    <Settings />
                    Account Settings
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={() => logout()}>
                  <LogOut />
                  Sign Out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
