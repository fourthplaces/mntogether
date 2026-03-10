import Link from "next/link";

export default function ContactPage() {
  return (
    <section className="page-section page-section--narrow">
      <Link href="/" className="back-link">
        &larr; Back to Home
      </Link>

      <h1 className="page-title" style={{ marginBottom: "2rem" }}>Contact Us</h1>

      <div className="about-content">
        <p>
          Have a question, want to submit a resource, or interested in partnering
          with MN Together? We&apos;d love to hear from you.
        </p>

        <div className="contact-card">
          <div>
            <h2 className="contact-section-title">Email</h2>
            <a
              href="mailto:hello@mntogether.org"
              className="body-link"
            >
              hello@mntogether.org
            </a>
          </div>

          <div>
            <h2 className="contact-section-title">Submit a Resource</h2>
            <p className="text-secondary" style={{ marginBottom: "0.75rem" }}>
              Know of a service, organization, or event that should be listed?
              Let us know and we&apos;ll review it for inclusion.
            </p>
            <a
              href="mailto:hello@mntogether.org?subject=Resource Submission"
              className="btn-primary"
            >
              Submit a Resource
            </a>
          </div>

          <div>
            <h2 className="contact-section-title">Partnerships</h2>
            <p className="text-secondary">
              If you represent an organization and want to reach more people in
              Minneapolis, reach out to discuss how we can work together.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
