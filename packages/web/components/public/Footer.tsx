import Link from "next/link";

export function Footer() {
  return (
    <footer className="bg-action text-border-strong px-6 md:px-12 pt-12 pb-8 mt-16">
      <div className="max-w-[1200px] mx-auto grid grid-cols-2 md:grid-cols-4 gap-8">
        <div>
          <h5 className="mb-4 text-surface font-bold">About</h5>
          <Link href="/about" className="block text-border-strong hover:text-surface transition-colors mb-2">Our Mission</Link>
          <Link href="/about" className="block text-border-strong hover:text-surface transition-colors mb-2">How It Works</Link>
          <Link href="/contact" className="block text-border-strong hover:text-surface transition-colors mb-2">Contact Us</Link>
        </div>
        <div>
          <h5 className="mb-4 text-surface font-bold">Get Involved</h5>
          <Link href="/posts?post_type=offering" className="block text-border-strong hover:text-surface transition-colors mb-2">Volunteer</Link>
          <Link href="/submit" className="block text-border-strong hover:text-surface transition-colors mb-2">Submit a Resource</Link>
          <Link href="/submit" className="block text-border-strong hover:text-surface transition-colors mb-2">Submit an Event</Link>
        </div>
        <div>
          <h5 className="mb-4 text-surface font-bold">Resources</h5>
          <Link href="/posts?post_type=seeking" className="block text-border-strong hover:text-surface transition-colors mb-2">Find Help</Link>
          <Link href="/organizations" className="block text-border-strong hover:text-surface transition-colors mb-2">Local Organizations</Link>
          <Link href="/posts?post_type=announcement" className="block text-border-strong hover:text-surface transition-colors mb-2">Community Events</Link>
        </div>
        <div>
          <h5 className="mb-4 text-surface font-bold">Information</h5>
          <Link href="/about" className="block text-border-strong hover:text-surface transition-colors mb-2">Privacy Policy</Link>
          <Link href="/about" className="block text-border-strong hover:text-surface transition-colors mb-2">Accessibility</Link>
          <Link href="/about" className="block text-border-strong hover:text-surface transition-colors mb-2">Know Your Rights</Link>
        </div>
      </div>
      <div className="text-center mt-8 pt-8 border-t border-text-secondary text-text-muted">
        <p>&copy; 2026 MN Together &bull; A community resource for Minneapolis</p>
      </div>
    </footer>
  );
}
