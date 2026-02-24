import Link from "next/link";

interface BackLinkProps {
  href: string;
  children: React.ReactNode;
  className?: string;
}

export function BackLink({ href, children, className = "" }: BackLinkProps) {
  return (
    <Link
      href={href}
      className={`text-sm text-text-muted hover:text-text-primary mb-6 inline-block ${className}`}
    >
      &larr; {children}
    </Link>
  );
}
