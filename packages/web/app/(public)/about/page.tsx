import Link from "next/link";

export default function AboutPage() {
  return (
    <section className="max-w-[800px] mx-auto px-6 md:px-12 pt-12 pb-24">
      <Link
        href="/"
        className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-8"
      >
        &larr; Back to Home
      </Link>

      <h1 className="text-4xl font-bold text-[#3D3D3D] mb-4">About MN Together</h1>

      <div className="space-y-6 text-[#4D4D4D] text-base leading-relaxed">
        <p className="text-lg">
          MN Together is a community hub connecting Minneapolis neighbors with
          resources, support, and each other. Find help when you need it, offer
          support when you can, and stay connected to your community.
        </p>

        <h2 className="text-2xl font-bold text-[#3D3D3D] pt-4">What We Do</h2>
        <p>We bring together essential resources in one accessible place:</p>
        <p>
          <strong className="text-[#3D3D3D]">For those seeking help</strong> — Find
          food assistance, shelter, legal aid, healthcare, and other vital
          services. No judgment, no barriers.
        </p>
        <p>
          <strong className="text-[#3D3D3D]">For those wanting to give</strong> — Discover
          volunteer opportunities and ways to support your neighbors through
          time, donations, or skills.
        </p>
        <p>
          <strong className="text-[#3D3D3D]">For staying connected</strong> — Learn
          about community events, gatherings, and opportunities to come together.
        </p>
        <p>
          <strong className="text-[#3D3D3D]">For service workers</strong> — Access
          professional tools, referral resources, and multilingual materials to
          better serve your clients.
        </p>

        <h2 className="text-2xl font-bold text-[#3D3D3D] pt-4">Why We Exist</h2>
        <p>
          During times of uncertainty, people need a trusted place to turn. MN
          Together was created to be that place — where anyone can find help,
          offer support, or simply connect with their community.
        </p>
        <p>
          We know that asking for help can be hard. We know that finding the
          right resources can be overwhelming. We&apos;re here to make it easier.
        </p>

        <h2 className="text-2xl font-bold text-[#3D3D3D] pt-4">Contact Us</h2>
        <p>
          Questions, suggestions, or want to get involved?
        </p>
        <Link
          href="/contact"
          className="inline-block mt-2 px-6 py-3 rounded-full bg-[#3D3D3D] text-white font-semibold text-sm hover:bg-[#2D2D2D] transition-colors"
        >
          Reach Out
        </Link>
      </div>
    </section>
  );
}
