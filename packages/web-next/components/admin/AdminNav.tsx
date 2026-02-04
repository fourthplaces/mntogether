"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const navItems = [
  { href: "/admin/dashboard", label: "Dashboard", icon: "\u{1F4CA}" },
  { href: "/admin/posts", label: "Posts", icon: "\u{1F4DD}" },
  { href: "/admin/scraped", label: "Scraped", icon: "\u{1F50D}" },
  { href: "/admin/websites", label: "Websites", icon: "\u{1F310}" },
  { href: "/admin/resources", label: "Resources", icon: "\u{1F4DA}" },
  { href: "/admin/organizations", label: "Organizations", icon: "\u{1F3E2}" },
  { href: "/admin/extraction", label: "Extraction", icon: "\u{2699}\u{FE0F}" },
];

export function AdminNav() {
  const pathname = usePathname();

  return (
    <nav className="bg-white border-b border-stone-200">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          <div className="flex items-center">
            <Link href="/admin/dashboard" className="flex items-center gap-2">
              <span className="text-2xl font-bold text-amber-600">MN Together</span>
              <span className="text-xs bg-amber-100 text-amber-700 px-2 py-1 rounded-full font-medium">
                Admin
              </span>
            </Link>
          </div>

          <div className="hidden md:block">
            <div className="flex items-center space-x-1">
              {navItems.map((item) => {
                const isActive =
                  pathname === item.href || pathname.startsWith(`${item.href}/`);
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={`flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      isActive
                        ? "bg-amber-100 text-amber-800"
                        : "text-stone-600 hover:bg-stone-100 hover:text-stone-900"
                    }`}
                  >
                    <span>{item.icon}</span>
                    <span>{item.label}</span>
                  </Link>
                );
              })}
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Link
              href="/"
              className="text-sm text-stone-500 hover:text-stone-700 px-3 py-2"
            >
              View Site
            </Link>
          </div>
        </div>
      </div>

      {/* Mobile nav */}
      <div className="md:hidden border-t border-stone-200">
        <div className="px-2 py-2 flex flex-wrap gap-1">
          {navItems.map((item) => {
            const isActive =
              pathname === item.href || pathname.startsWith(`${item.href}/`);
            return (
              <Link
                key={item.href}
                href={item.href}
                className={`flex items-center gap-1 px-2 py-1 rounded text-xs font-medium transition-colors ${
                  isActive
                    ? "bg-amber-100 text-amber-800"
                    : "text-stone-600 hover:bg-stone-100"
                }`}
              >
                <span>{item.icon}</span>
                <span>{item.label}</span>
              </Link>
            );
          })}
        </div>
      </div>
    </nav>
  );
}
