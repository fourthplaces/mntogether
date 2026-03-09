import Link from "next/link";

export function Header() {
  return (
    <header className="site-header">
      <Link href="/" className="site-header-logo">
        MN Together
      </Link>
      <nav className="site-nav">
        <Link href="/about" className="nav-link">About</Link>
        <Link href="/organizations" className="nav-link">Organizations</Link>
        <Link href="/contact" className="nav-link">Contact</Link>
      </nav>
    </header>
  );
}
