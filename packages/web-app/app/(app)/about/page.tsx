import Link from "next/link";

export default function AboutPage() {
  return (
    <section className="page-section page-section--narrow">
      <Link href="/" className="back-link">
        &larr; Back to Home
      </Link>

      <h1 className="page-title" style={{ marginBottom: "1rem" }}>About MN Together</h1>

      <div className="about-content">
        <p className="about-lead">
          MN Together is a community hub connecting Minneapolis neighbors with
          resources, support, and each other. Find help when you need it, offer
          support when you can, and stay connected to your community.
        </p>

        <h2>What We Do</h2>
        <p>We bring together essential resources in one accessible place:</p>
        <p>
          <strong>For those seeking help</strong> — Find
          food assistance, shelter, legal aid, healthcare, and other vital
          services. No judgment, no barriers.
        </p>
        <p>
          <strong>For those wanting to give</strong> — Discover
          volunteer opportunities and ways to support your neighbors through
          time, donations, or skills.
        </p>
        <p>
          <strong>For staying connected</strong> — Learn
          about community events, gatherings, and opportunities to come together.
        </p>
        <p>
          <strong>For service workers</strong> — Access
          professional tools, referral resources, and multilingual materials to
          better serve your clients.
        </p>

        <h2>Why We Exist</h2>
        <p>
          During times of uncertainty, people need a trusted place to turn. MN
          Together was created to be that place — where anyone can find help,
          offer support, or simply connect with their community.
        </p>
        <p>
          We know that asking for help can be hard. We know that finding the
          right resources can be overwhelming. We&apos;re here to make it easier.
        </p>

        <h2>Contact Us</h2>
        <p>
          Questions, suggestions, or want to get involved?
        </p>
        <Link
          href="/contact"
          className="btn-primary"
          style={{ marginTop: "0.5rem" }}
        >
          Reach Out
        </Link>
      </div>
    </section>
  );
}
