import Link from "next/link";

export function Footer() {
  return (
    <footer className="border-t border-border px-4 pt-8 pb-6 mt-10">
      <div className="max-w-[960px] mx-auto grid grid-cols-2 md:grid-cols-4 gap-6 text-sm">
        <div>
          <h5 className="mb-2 font-bold text-text-primary">About</h5>
          <Link href="/about" className="block text-text-secondary hover:underline mb-1">Our Mission</Link>
          <Link href="/about" className="block text-text-secondary hover:underline mb-1">How It Works</Link>
          <Link href="/contact" className="block text-text-secondary hover:underline mb-1">Contact Us</Link>
        </div>
        <div>
          <h5 className="mb-2 font-bold text-text-primary">Get Involved</h5>
          <Link href="/posts?post_type=offering" className="block text-text-secondary hover:underline mb-1">Volunteer</Link>
        </div>
        <div>
          <h5 className="mb-2 font-bold text-text-primary">Resources</h5>
          <Link href="/posts?post_type=seeking" className="block text-text-secondary hover:underline mb-1">Find Help</Link>
          <Link href="/organizations" className="block text-text-secondary hover:underline mb-1">Local Organizations</Link>
          <Link href="/posts?post_type=announcement" className="block text-text-secondary hover:underline mb-1">Community Events</Link>
        </div>
        <div>
          <h5 className="mb-2 font-bold text-text-primary">Information</h5>
          <Link href="/about" className="block text-text-secondary hover:underline mb-1">Privacy Policy</Link>
          <Link href="/about" className="block text-text-secondary hover:underline mb-1">Accessibility</Link>
          <Link href="/about" className="block text-text-secondary hover:underline mb-1">Know Your Rights</Link>
        </div>
      </div>
      <div className="text-center mt-6 pt-4 border-t border-border text-text-muted text-xs">
        <p>&copy; 2026 MN Together</p>
      </div>
    </footer>
  );
}
