import Link from "next/link";

export function Footer() {
  return (
    <footer className="site-footer">
      <div className="footer-grid">
        <div>
          <h5 className="footer-heading">About</h5>
          <Link href="/about" className="footer-link">Our Mission</Link>
          <Link href="/about" className="footer-link">How It Works</Link>
          <Link href="/contact" className="footer-link">Contact Us</Link>
        </div>
        <div>
          <h5 className="footer-heading">Get Involved</h5>
          <Link href="/posts?post_type=offering" className="footer-link">Volunteer</Link>
        </div>
        <div>
          <h5 className="footer-heading">Resources</h5>
          <Link href="/posts?post_type=seeking" className="footer-link">Find Help</Link>
          <Link href="/organizations" className="footer-link">Local Organizations</Link>
          <Link href="/posts?post_type=announcement" className="footer-link">Community Events</Link>
        </div>
        <div>
          <h5 className="footer-heading">Information</h5>
          <Link href="/about" className="footer-link">Privacy Policy</Link>
          <Link href="/about" className="footer-link">Accessibility</Link>
          <Link href="/about" className="footer-link">Know Your Rights</Link>
        </div>
      </div>
      <div className="footer-copyright">
        <p>&copy; 2026 MN Together</p>
      </div>
    </footer>
  );
}
