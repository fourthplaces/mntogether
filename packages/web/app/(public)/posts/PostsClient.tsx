"use client";

import { useMemo } from "react";
import Link from "next/link";
import { useSearchParams, useRouter } from "next/navigation";
import { useRestate } from "@/lib/restate/client";
import type {
  PublicListResult,
  PublicFiltersResult,
  PublicPostResult,
} from "@/lib/restate/types";

function getPostTagStyle(postType: string): {
  bg: string;
  text: string;
  label: string;
} {
  switch (postType) {
    case "help":
      return { bg: "bg-[#F4D9B8]", text: "text-[#8B6D3F]", label: "Help" };
    case "opportunities":
      return {
        bg: "bg-[#B8CFC4]",
        text: "text-[#4D6B5F]",
        label: "Support",
      };
    case "event":
      return {
        bg: "bg-[#D4C4E8]",
        text: "text-[#6D5B8B]",
        label: "Community",
      };
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
        <p className="text-sm text-[#7D7D7D] mb-1">{post.location}</p>
      )}
      <p className="text-[#5D5D5D] text-[0.95rem] leading-relaxed mb-3">
        {post.tldr || post.description}
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

export function PostsClient() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const postType = searchParams.get("post_type");

  const requestBody = useMemo(() => {
    const body: Record<string, unknown> = {};
    if (postType) body.post_type = postType;
    return body;
  }, [postType]);

  const { data: listData, isLoading: listLoading } =
    useRestate<PublicListResult>("Posts", "public_list", requestBody);

  const { data: filtersData } =
    useRestate<PublicFiltersResult>("Posts", "public_filters", {});

  const posts = listData?.posts ?? [];
  const postTypes = filtersData?.post_types ?? [];

  const setFilter = (value: string | null) => {
    const params = new URLSearchParams();
    if (value) params.set("post_type", value);
    router.replace(`/posts${params.toString() ? `?${params}` : ""}`);
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

        {/* Title + Filters */}
        <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-8 gap-4">
          <h1 className="text-3xl font-bold text-[#3D3D3D]">
            Community Resources
          </h1>
          <div className="flex gap-3 overflow-x-auto scrollbar-hide">
            <button
              onClick={() => setFilter(null)}
              className={`px-5 py-2 rounded-full text-sm font-semibold border transition-all ${
                postType === null
                  ? "bg-[#3D3D3D] text-white border-[#3D3D3D]"
                  : "bg-transparent text-[#5D5D5D] border-[#C4B8A0] hover:border-[#3D3D3D]"
              }`}
            >
              All
            </button>
            {postTypes.map((pt) => (
              <button
                key={pt.value}
                onClick={() =>
                  setFilter(postType === pt.value ? null : pt.value)
                }
                className={`px-5 py-2 rounded-full text-sm font-semibold border transition-all whitespace-nowrap ${
                  postType === pt.value
                    ? "bg-[#3D3D3D] text-white border-[#3D3D3D]"
                    : "bg-transparent text-[#5D5D5D] border-[#C4B8A0] hover:border-[#3D3D3D]"
                }`}
              >
                {pt.display_name}
              </button>
            ))}
          </div>
        </div>

        {/* Post List */}
        <div className="flex flex-col gap-4">
          {listLoading ? (
            Array.from({ length: 8 }).map((_, i) => (
              <PostCardSkeleton key={i} />
            ))
          ) : posts.length === 0 ? (
            <div className="py-16 text-center">
              <p className="text-[#7D7D7D] text-sm">
                No resources found. Try a different filter.
              </p>
            </div>
          ) : (
            posts.map((post) => <PostCard key={post.id} post={post} />)
          )}
        </div>

        {/* Result count */}
        {!listLoading && posts.length > 0 && (
          <p className="text-center text-sm text-[#7D7D7D] mt-6">
            Showing {posts.length} of {listData?.total_count ?? posts.length}{" "}
            results
          </p>
        )}
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
