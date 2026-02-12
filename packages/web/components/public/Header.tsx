import Link from "next/link";

export function Header() {
  return (
    <header className="bg-surface px-6 md:px-12 py-6 flex justify-between items-center">
      <Link href="/" className="flex items-center gap-2 text-2xl font-bold text-text-primary">
        MN <img src="/icon-mn.svg" alt="Minnesota" className="w-5 h-5" /> Together
      </Link>
      <nav className="hidden md:flex gap-10 items-center">
        <Link href="/about" className="text-text-primary font-medium">About</Link>
        <Link href="/organizations" className="text-text-primary font-medium">Organizations</Link>
        <Link href="/contact" className="text-text-primary font-medium">Contact</Link>
      </nav>
    </header>
  );
}
