import Link from "next/link";

export default function AboutPage() {
  return (
    <div className="min-h-screen bg-[#E8E2D5] text-[#3D3D3D]">
      {/* Header */}
      <header className="bg-[#E8E2D5] px-6 md:px-12 py-6 flex justify-between items-center">
        <Link
          href="/"
          className="flex items-center gap-2 text-2xl font-bold text-[#3D3D3D]"
        >
          MN{" "}
          <img src="/icon-mn.svg" alt="Minnesota" className="w-5 h-5" />{" "}
          Together
        </Link>
        <nav className="hidden md:flex gap-10 items-center">
          <Link href="/about" className="text-[#3D3D3D] font-medium">About</Link>
          <Link href="/posts" className="text-[#3D3D3D] font-medium">Resources</Link>
          <Link href="/contact" className="text-[#3D3D3D] font-medium">Contact</Link>
        </nav>
      </header>

      {/* Content */}
      <section className="max-w-[800px] mx-auto px-6 md:px-12 pt-12 pb-24">
        <Link
          href="/"
          className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-8"
        >
          &larr; Back to Home
        </Link>

        <h1 className="text-4xl font-bold text-[#3D3D3D] mb-8">About MN Together</h1>

        <div className="space-y-6 text-[#4D4D4D] text-base leading-relaxed">
          <p>
            MN Together is a community resource hub connecting Minneapolis residents
            with the services, volunteer opportunities, and local organizations that
            strengthen our neighborhoods.
          </p>

          <h2 className="text-2xl font-bold text-[#3D3D3D] pt-4">Our Mission</h2>
          <p>
            We believe that strong communities are built when people can easily find
            and share resources. Whether you need help, want to volunteer, or are
            looking for ways to connect with your neighbors, MN Together makes it
            simple to get involved.
          </p>

          <h2 className="text-2xl font-bold text-[#3D3D3D] pt-4">How It Works</h2>
          <p>
            We aggregate resources from trusted local organizations and community
            members across Minneapolis. Our platform makes it easy to discover
            services, volunteer opportunities, and community events — all in one place.
          </p>

          <h2 className="text-2xl font-bold text-[#3D3D3D] pt-4">Get Involved</h2>
          <p>
            MN Together is built by and for the community. If you know of a resource
            that should be listed, or if you represent an organization that wants to
            reach more people, we&apos;d love to hear from you.
          </p>
          <Link
            href="/contact"
            className="inline-block mt-2 px-6 py-3 rounded-full bg-[#3D3D3D] text-white font-semibold text-sm hover:bg-[#2D2D2D] transition-colors"
          >
            Contact Us
          </Link>
        </div>
      </section>
    </div>
  );
}
