/**
 * Broadsheet layout — full-bleed newspaper frame, no site chrome.
 * Used for post detail pages where the newspaper IS the page.
 */
import { SiteFooter } from "@/components/broadsheet/chrome/SiteFooter";

export default function BroadsheetLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div style={{ backgroundColor: "var(--deep-forest)", minHeight: "100vh" }}>
      {children}
      <SiteFooter />
    </div>
  );
}
