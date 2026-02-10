"use client";

import Link from "next/link";
import { useSearchParams, useRouter } from "next/navigation";
import { PostFeed } from "@/components/public/PostFeed";

export function PostsClient() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const postType = searchParams.get("post_type");

  const setFilter = (value: string | null) => {
    const params = new URLSearchParams();
    if (value) {
      params.set("post_type", value);
    } else {
      params.set("post_type", "all");
    }
    router.replace(`/posts?${params}`);
  };

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
          <a href="/#about" className="text-[#3D3D3D] font-medium">
            About
          </a>
          <a href="/#resources" className="text-[#3D3D3D] font-medium">
            Resources
          </a>
          <a href="/#contact" className="text-[#3D3D3D] font-medium">
            Contact
          </a>
        </nav>
      </header>

      {/* Content */}
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
        {/* Back link */}
        <Link
          href="/"
          className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-6"
        >
          &larr; Back to Home
        </Link>

        <PostFeed
          title="Community Resources"
          activePostType={postType}
          onFilterChange={setFilter}
          showResultCount
          skeletonCount={8}
        />
      </section>

      {/* Footer */}
      <footer className="bg-[#3D3D3D] text-[#C4B8A0] px-6 md:px-12 pt-12 pb-8 mt-16">
        <div className="max-w-[1200px] mx-auto grid grid-cols-2 md:grid-cols-4 gap-8">
          <div>
            <h5 className="mb-4 text-[#E8E2D5] font-bold">About</h5>
            <a href="/#mission" className="block text-[#C4B8A0] mb-2">
              Our Mission
            </a>
            <a href="/#how-it-works" className="block text-[#C4B8A0] mb-2">
              How It Works
            </a>
            <a href="/#contact" className="block text-[#C4B8A0] mb-2">
              Contact Us
            </a>
          </div>
          <div>
            <h5 className="mb-4 text-[#E8E2D5] font-bold">Get Involved</h5>
            <a href="/#volunteer" className="block text-[#C4B8A0] mb-2">
              Volunteer
            </a>
            <a href="/#submit" className="block text-[#C4B8A0] mb-2">
              Submit a Resource
            </a>
            <a href="/#events" className="block text-[#C4B8A0] mb-2">
              Submit an Event
            </a>
          </div>
          <div>
            <h5 className="mb-4 text-[#E8E2D5] font-bold">Resources</h5>
            <a href="/#help" className="block text-[#C4B8A0] mb-2">
              Find Help
            </a>
            <a href="/#businesses" className="block text-[#C4B8A0] mb-2">
              Local Businesses
            </a>
            <a href="/#calendar" className="block text-[#C4B8A0] mb-2">
              Event Calendar
            </a>
          </div>
          <div>
            <h5 className="mb-4 text-[#E8E2D5] font-bold">Information</h5>
            <a href="/#privacy" className="block text-[#C4B8A0] mb-2">
              Privacy Policy
            </a>
            <a href="/#accessibility" className="block text-[#C4B8A0] mb-2">
              Accessibility
            </a>
            <a href="/#rights" className="block text-[#C4B8A0] mb-2">
              Know Your Rights
            </a>
          </div>
        </div>
        <div className="text-center mt-8 pt-8 border-t border-[#5D5D5D] text-[#999]">
          <p>
            &copy; 2026 MN Together &bull; A community resource for Minneapolis
          </p>
        </div>
      </footer>
    </div>
  );
}
