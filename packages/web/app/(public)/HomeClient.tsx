"use client";

import { useState } from "react";
import Link from "next/link";
import { useRestate } from "@/lib/restate/client";
import { SubmitSheet } from "@/components/public/SubmitSheet";
import type {
  PublicListResult,
  PublicFiltersResult,
  PublicPostResult,
} from "@/lib/restate/types";

type ActiveSheet = "search" | "submit" | null;

function getPostTagStyle(postType: string): { bg: string; text: string; label: string } {
  switch (postType) {
    case "help":
      return { bg: "bg-[#F4D9B8]", text: "text-[#8B6D3F]", label: "Help" };
    case "opportunities":
      return { bg: "bg-[#B8CFC4]", text: "text-[#4D6B5F]", label: "Support" };
    case "event":
      return { bg: "bg-[#D4C4E8]", text: "text-[#6D5B8B]", label: "Community" };
    case "professional":
      return { bg: "bg-[#E6B8B8]", text: "text-[#8B4D4D]", label: "Event" };
    default:
      return { bg: "bg-[#F4D9B8]", text: "text-[#8B6D3F]", label: "Help" };
  }
}

function formatCategory(value: string): string {
  return value
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

function PostCard({ post }: { post: PublicPostResult }) {
  const tagStyle = getPostTagStyle(post.post_type);
  const serviceOfferedTags = post.tags.filter((t) => t.kind === "service_offered");

  return (
    <Link
      href={`/posts/${post.id}`}
      className="bg-white p-6 rounded-lg border border-[#E8DED2] hover:shadow-md transition-shadow block"
    >
      <h3 className="text-xl font-bold text-[#3D3D3D] mb-1">{post.title}</h3>
      {post.location && (
        <p className="text-sm text-[#7D7D7D] mb-1">📍 {post.location}</p>
      )}
      <p className="text-[#5D5D5D] text-[0.95rem] leading-relaxed mb-3">
        {post.summary || post.description}
      </p>
      <div className="flex flex-wrap gap-2">
        <span
          className={`inline-block px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wide ${tagStyle.bg} ${tagStyle.text}`}
        >
          {tagStyle.label}
        </span>
        {serviceOfferedTags.map((tag) => (
          <span
            key={tag.value}
            className="px-3 py-1 rounded-full text-xs font-medium bg-[#F5F1E8] text-[#5D5D5D]"
          >
            {tag.display_name || formatCategory(tag.value)}
          </span>
        ))}
      </div>
    </Link>
  );
}

function PostCardSkeleton() {
  return (
    <div className="bg-white p-6 rounded-lg border border-[#E8DED2] animate-pulse">
      <div className="h-6 w-3/4 bg-gray-200 rounded mb-2" />
      <div className="h-4 w-1/3 bg-gray-200 rounded mb-2" />
      <div className="h-4 w-full bg-gray-200 rounded mb-1" />
      <div className="h-4 w-5/6 bg-gray-200 rounded mb-3" />
      <div className="h-6 w-20 bg-gray-200 rounded-full" />
    </div>
  );
}

export function HomeClient() {
  const [activeSheet, setActiveSheet] = useState<ActiveSheet>(null);

  const { data: listData, isLoading: listLoading } =
    useRestate<PublicListResult>("Posts", "public_list", {});

  const { data: filtersData } =
    useRestate<PublicFiltersResult>("Posts", "public_filters", {});

  const posts = listData?.posts ?? [];
  const postTypes = filtersData?.post_types ?? [];

  return (
    <div className="min-h-screen bg-[#E8E2D5] text-[#3D3D3D] relative leading-relaxed">
      {/* Skyline background */}
      <div
        className="absolute inset-0 w-screen h-screen z-0 opacity-50 pointer-events-none"
        style={{
          backgroundImage: "url('/skyline.png')",
          backgroundPosition: "center 100px",
          backgroundRepeat: "no-repeat",
          backgroundSize: "80%",
        }}
      />

      {/* Header */}
      <header className="bg-[#E8E2D5] px-6 md:px-12 py-6 flex justify-between items-center relative z-[100]">
        <div className="flex items-center gap-2 text-2xl font-bold text-[#3D3D3D]">
          MN <img src="/icon-mn.svg" alt="Minnesota" className="w-5 h-5" /> Together
        </div>
        <nav className="hidden md:flex gap-10 items-center">
          {process.env.NODE_ENV === "development" && (
            <Link href="/about" className="text-[#3D3D3D] font-medium">About</Link>
          )}
          <Link href="/posts" className="text-[#3D3D3D] font-medium">Resources</Link>
          {process.env.NODE_ENV === "development" && (
            <Link href="/contact" className="text-[#3D3D3D] font-medium">Contact</Link>
          )}
        </nav>
      </header>

      {/* Hero */}
      <section className="px-6 md:px-12 pt-16 pb-8 max-w-[1200px] mx-auto relative z-10">
        <h1 className="text-4xl sm:text-[3.5rem] font-bold text-[#3D3D3D] leading-tight max-w-[800px]">
          Strength in Community, Together.
        </h1>
      </section>

      {/* Pathway Cards */}
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-4 pb-16 grid grid-cols-1 md:grid-cols-3 gap-6 relative z-10">
        <Link href="/posts?post_type=help" className="bg-[#F4D9B8] border-2 border-[#E0C4A0] rounded-2xl p-8 flex flex-col cursor-pointer hover:bg-[#E0C4A0] hover:border-[#C9AD89] transition-all duration-300">
          <div className="flex items-center gap-3 mb-4">
            <img src="/icon-help.svg" alt="" className="w-10 h-10" />
            <h3 className="text-2xl font-bold text-[#3D3D3D]">I Need Help</h3>
          </div>
          <p className="text-[#4D4D4D] text-base leading-relaxed mb-6 flex-1">
            People looking for food, shelter, legal aid, and other resources. You&apos;re not alone.
          </p>
          <span className="bg-[#3D3D3D] text-white px-7 py-3 rounded-full text-[0.95rem] font-semibold self-start mt-auto">
            Find Help
          </span>
        </Link>

        <Link href="/posts?post_type=opportunities" className="bg-[#B8CFC4] border-2 border-[#A0BDB0] rounded-2xl p-8 flex flex-col cursor-pointer hover:bg-[#A0BDB0] hover:border-[#8AA89B] transition-all duration-300">
          <div className="flex items-center gap-3 mb-4">
            <img src="/icon-support.svg" alt="" className="w-10 h-10" />
            <h3 className="text-2xl font-bold text-[#3D3D3D]">I Want to Support</h3>
          </div>
          <p className="text-[#4D4D4D] text-base leading-relaxed mb-6 flex-1">
            Volunteer opportunities and ways to give back to neighbors who need it.
          </p>
          <span className="bg-[#3D3D3D] text-white px-7 py-3 rounded-full text-[0.95rem] font-semibold self-start mt-auto">
            Find Opportunities
          </span>
        </Link>

        <Link href="/posts?post_type=event" className="bg-[#E6B8B8] border-2 border-[#D4A0A0] rounded-2xl p-8 flex flex-col cursor-pointer hover:bg-[#D4A0A0] hover:border-[#C08989] transition-all duration-300">
          <div className="flex items-center gap-3 mb-4">
            <img src="/icon-events.svg" alt="" className="w-10 h-10" />
            <h3 className="text-2xl font-bold text-[#3D3D3D]">Community Events</h3>
          </div>
          <p className="text-[#4D4D4D] text-base leading-relaxed mb-6 flex-1">
            Food drives, gatherings, and opportunities to connect with your community.
          </p>
          <span className="bg-[#3D3D3D] text-white px-7 py-3 rounded-full text-[0.95rem] font-semibold self-start mt-auto">
            Explore Community Events
          </span>
        </Link>
      </section>

      {/* Posts Section */}
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-8 gap-4">
          <h2 className="text-3xl font-bold text-[#3D3D3D]">Get Involved</h2>
          <div className="flex gap-3 overflow-x-auto scrollbar-hide">
            <Link
              href="/posts"
              className="px-5 py-2 rounded-full text-sm font-semibold border transition-all bg-transparent text-[#5D5D5D] border-[#C4B8A0] hover:border-[#3D3D3D]"
            >
              All
            </Link>
            {postTypes.map((pt) => (
              <Link
                key={pt.value}
                href={`/posts?post_type=${pt.value}`}
                className="px-5 py-2 rounded-full text-sm font-semibold border transition-all whitespace-nowrap bg-transparent text-[#5D5D5D] border-[#C4B8A0] hover:border-[#3D3D3D]"
              >
                {pt.display_name}
              </Link>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-4">
          {listLoading ? (
            Array.from({ length: 6 }).map((_, i) => (
              <PostCardSkeleton key={i} />
            ))
          ) : posts.length === 0 ? (
            <div className="py-16 text-center">
              <p className="text-[#7D7D7D] text-sm">No resources found. Try a different filter.</p>
            </div>
          ) : (
            <>
              {posts.map((post) => <PostCard key={post.id} post={post} />)}
              <div className="text-center pt-4">
                <Link
                  href="/posts"
                  className="inline-block px-6 py-3 rounded-full border border-[#C4B8A0] text-[#5D5D5D] font-semibold text-sm hover:border-[#3D3D3D] hover:text-[#3D3D3D] transition-all"
                >
                  See More
                </Link>
              </div>
            </>
          )}
        </div>
      </section>

      {/* Footer — dev only until content is finalized */}
      {process.env.NODE_ENV === "development" && (
        <footer className="bg-[#3D3D3D] text-[#C4B8A0] px-6 md:px-12 pt-12 pb-8 mt-16">
          <div className="max-w-[1200px] mx-auto grid grid-cols-2 md:grid-cols-4 gap-8">
            <div>
              <h5 className="mb-4 text-[#E8E2D5] font-bold">About</h5>
              <a href="#mission" className="block text-[#C4B8A0] mb-2">Our Mission</a>
              <a href="#how-it-works" className="block text-[#C4B8A0] mb-2">How It Works</a>
              <a href="#contact" className="block text-[#C4B8A0] mb-2">Contact Us</a>
            </div>
            <div>
              <h5 className="mb-4 text-[#E8E2D5] font-bold">Get Involved</h5>
              <a href="#volunteer" className="block text-[#C4B8A0] mb-2">Volunteer</a>
              <a href="#submit" className="block text-[#C4B8A0] mb-2">Submit a Resource</a>
              <a href="#events" className="block text-[#C4B8A0] mb-2">Submit an Event</a>
            </div>
            <div>
              <h5 className="mb-4 text-[#E8E2D5] font-bold">Resources</h5>
              <a href="#help" className="block text-[#C4B8A0] mb-2">Find Help</a>
              <a href="#businesses" className="block text-[#C4B8A0] mb-2">Local Businesses</a>
              <a href="#calendar" className="block text-[#C4B8A0] mb-2">Event Calendar</a>
            </div>
            <div>
              <h5 className="mb-4 text-[#E8E2D5] font-bold">Information</h5>
              <a href="#privacy" className="block text-[#C4B8A0] mb-2">Privacy Policy</a>
              <a href="#accessibility" className="block text-[#C4B8A0] mb-2">Accessibility</a>
              <a href="#rights" className="block text-[#C4B8A0] mb-2">Know Your Rights</a>
            </div>
          </div>
          <div className="text-center mt-8 pt-8 border-t border-[#5D5D5D] text-[#999]">
            <p>&copy; 2026 MN Together &bull; A community resource for Minneapolis</p>
          </div>
        </footer>
      )}

      {/* Bottom Sheets */}
      <SubmitSheet isOpen={activeSheet === "submit"} onClose={() => setActiveSheet(null)} />
    </div>
  );
}
