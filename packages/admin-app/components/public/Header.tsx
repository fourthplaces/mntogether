import Link from "next/link";

export function Header() {
  return (
    <header className="bg-[#E8E2D5] px-6 md:px-12 py-6 flex justify-between items-center">
      <Link href="/" className="flex items-center gap-2 text-2xl font-bold text-[#3D3D3D]">
        MN <img src="/icon-mn.svg" alt="Minnesota" className="w-5 h-5" /> Together
      </Link>
      <nav className="hidden md:flex gap-10 items-center">
        <Link href="/about" className="text-[#3D3D3D] font-medium">About</Link>
        <Link href="/organizations" className="text-[#3D3D3D] font-medium">Organizations</Link>
        <Link href="/contact" className="text-[#3D3D3D] font-medium">Contact</Link>
      </nav>
    </header>
  );
}
