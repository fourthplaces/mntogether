import Link from "next/link";
import { ArrowLeft } from "lucide-react";

interface BackLinkProps {
  href: string;
  children: React.ReactNode;
  className?: string;
}

export function BackLink({ href, children, className = "" }: BackLinkProps) {
  return (
    <Link
      href={href}
      className={`text-sm text-text-muted hover:text-text-primary mb-6 inline-flex items-center gap-1 ${className}`}
    >
      <ArrowLeft className="w-4 h-4" /> {children}
    </Link>
  );
}
