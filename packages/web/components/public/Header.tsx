import Link from "next/link";

export function Header() {
  return (
    <header className="border-b border-border px-4 py-3 flex justify-between items-center">
      <Link href="/" className="font-bold text-text-primary">
        MN Together
      </Link>
      <nav className="hidden md:flex gap-6 items-center text-sm">
        <Link href="/about" className="text-text-primary hover:underline">About</Link>
        <Link href="/organizations" className="text-text-primary hover:underline">Organizations</Link>
        <Link href="/contact" className="text-text-primary hover:underline">Contact</Link>
      </nav>
    </header>
  );
}
