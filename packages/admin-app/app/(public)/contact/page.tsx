import Link from "next/link";

export default function ContactPage() {
  return (
    <section className="max-w-[800px] mx-auto px-6 md:px-12 pt-12 pb-24">
      <Link
        href="/"
        className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-8"
      >
        &larr; Back to Home
      </Link>

      <h1 className="text-4xl font-bold text-[#3D3D3D] mb-8">Contact Us</h1>

      <div className="space-y-6 text-[#4D4D4D] text-base leading-relaxed">
        <p>
          Have a question, want to submit a resource, or interested in partnering
          with MN Together? We&apos;d love to hear from you.
        </p>

        <div className="bg-white rounded-lg border border-[#E8DED2] p-8 space-y-6">
          <div>
            <h2 className="text-xl font-bold text-[#3D3D3D] mb-2">Email</h2>
            <a
              href="mailto:hello@mntogether.org"
              className="text-[#5D5D5D] hover:text-[#3D3D3D] underline"
            >
              hello@mntogether.org
            </a>
          </div>

          <div>
            <h2 className="text-xl font-bold text-[#3D3D3D] mb-2">Submit a Resource</h2>
            <p className="text-[#5D5D5D] mb-3">
              Know of a service, organization, or event that should be listed?
              Let us know and we&apos;ll review it for inclusion.
            </p>
            <a
              href="mailto:hello@mntogether.org?subject=Resource Submission"
              className="inline-block px-6 py-3 rounded-full bg-[#3D3D3D] text-white font-semibold text-sm hover:bg-[#2D2D2D] transition-colors"
            >
              Submit a Resource
            </a>
          </div>

          <div>
            <h2 className="text-xl font-bold text-[#3D3D3D] mb-2">Partnerships</h2>
            <p className="text-[#5D5D5D]">
              If you represent an organization and want to reach more people in
              Minneapolis, reach out to discuss how we can work together.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
