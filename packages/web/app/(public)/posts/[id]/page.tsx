"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useRestateObject, callObject } from "@/lib/restate/client";
import { useEffect } from "react";
import type { PostResult, TagResult } from "@/lib/restate/types";
import CommentsSection from "@/components/public/CommentsSection";

function getPostTagStyle(postType: string): {
  bg: string;
  text: string;
  label: string;
} {
  switch (postType) {
    case "service":
      return { bg: "bg-[#F4D9B8]", text: "text-[#8B6D3F]", label: "Help" };
    case "opportunity":
      return { bg: "bg-[#B8CFC4]", text: "text-[#4D6B5F]", label: "Support" };
    case "business":
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

function formatTimeAgo(dateString: string) {
  const date = new Date(dateString);
  const now = new Date();
  const diffInDays = Math.floor(
    (now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24)
  );
  if (diffInDays === 0) return "Today";
  if (diffInDays === 1) return "Yesterday";
  if (diffInDays < 7) return `${diffInDays} days ago`;
  if (diffInDays < 30) return `${Math.floor(diffInDays / 7)} weeks ago`;
  return `${Math.floor(diffInDays / 30)} months ago`;
}

export default function PublicPostDetailPage() {
  const params = useParams();
  const postId = params.id as string;

  const { data: post, isLoading } = useRestateObject<PostResult>(
    "Post",
    postId,
    "get",
    {}
  );

  // Track view
  useEffect(() => {
    if (postId) {
      callObject("Post", postId, "track_view", {}).catch(() => {});
    }
  }, [postId]);

  const handleSourceClick = () => {
    callObject("Post", postId, "track_click", {}).catch(() => {});
  };

  if (isLoading) {
    return (
      <div className="min-h-screen bg-[#E8E2D5]">
        <header className="bg-[#E8E2D5] px-6 md:px-12 py-6 flex justify-between items-center">
          <Link href="/" className="flex items-center gap-2 text-2xl font-bold text-[#3D3D3D]">
            MN <img src="/icon-mn.svg" alt="Minnesota" className="w-5 h-5" /> Together
          </Link>
          <nav className="hidden md:flex gap-10 items-center">
            <Link href="/posts" className="text-[#3D3D3D] font-medium">Resources</Link>
          </nav>
        </header>
        <div className="max-w-[800px] mx-auto px-6 md:px-12 pt-8 pb-16 animate-pulse">
          <div className="h-4 w-24 bg-[#D4CEC1] rounded mb-8" />
          <div className="bg-white rounded-lg border border-[#E8DED2] p-8">
            <div className="h-8 w-3/4 bg-gray-200 rounded mb-4" />
            <div className="h-4 w-1/3 bg-gray-200 rounded mb-6" />
            <div className="h-16 w-full bg-gray-100 rounded mb-6" />
            <div className="space-y-3">
              <div className="h-4 w-full bg-gray-200 rounded" />
              <div className="h-4 w-5/6 bg-gray-200 rounded" />
              <div className="h-4 w-4/6 bg-gray-200 rounded" />
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-[#E8E2D5]">
        <header className="bg-[#E8E2D5] px-6 md:px-12 py-6 flex justify-between items-center">
          <Link href="/" className="flex items-center gap-2 text-2xl font-bold text-[#3D3D3D]">
            MN <img src="/icon-mn.svg" alt="Minnesota" className="w-5 h-5" /> Together
          </Link>
        </header>
        <div className="max-w-[800px] mx-auto px-6 md:px-12 pt-16 text-center">
          <h1 className="text-xl font-semibold text-[#3D3D3D] mb-2">
            Post not found
          </h1>
          <Link href="/" className="text-[#7D7D7D] hover:text-[#3D3D3D] text-sm">
            Back to home
          </Link>
        </div>
      </div>
    );
  }

  const tags = post.tags || [];
  const tagStyle = post.post_type ? getPostTagStyle(post.post_type) : null;
  const serviceOfferedTags = tags.filter(
    (t: TagResult) => t.kind === "service_offered"
  );

  return (
    <div className="min-h-screen bg-[#E8E2D5] text-[#3D3D3D]">
      {/* Header */}
      <header className="bg-[#E8E2D5] px-6 md:px-12 py-6 flex justify-between items-center">
        <Link href="/" className="flex items-center gap-2 text-2xl font-bold text-[#3D3D3D]">
          MN <img src="/icon-mn.svg" alt="Minnesota" className="w-5 h-5" /> Together
        </Link>
        <nav className="hidden md:flex gap-10 items-center">
          <Link href="/posts" className="text-[#3D3D3D] font-medium">Resources</Link>
        </nav>
      </header>

      {/* Content */}
      <section className="max-w-[800px] mx-auto px-6 md:px-12 pt-8 pb-16">
        {/* Back link */}
        <Link
          href="/posts"
          className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-6"
        >
          &larr; Back to Resources
        </Link>

        <div className="bg-white rounded-lg border border-[#E8DED2] p-8">
          {/* Title */}
          <h1 className="text-2xl sm:text-3xl font-bold text-[#3D3D3D] mb-3">
            {post.title}
          </h1>

          {/* Meta row */}
          <div className="flex flex-wrap items-center gap-3 text-sm text-[#7D7D7D] mb-6">
            {post.location && (
              <span className="inline-flex items-center gap-1">
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
                {post.location}
              </span>
            )}
            <span>Posted {formatTimeAgo(post.created_at)}</span>
          </div>

          {/* Tags */}
          <div className="flex flex-wrap gap-2 mb-6">
            {tagStyle && (
              <span className={`inline-block px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wide ${tagStyle.bg} ${tagStyle.text}`}>
                {tagStyle.label}
              </span>
            )}
            {serviceOfferedTags.map((tag: TagResult) => (
              <span
                key={tag.id}
                className="px-3 py-1 rounded-full text-xs font-medium bg-[#F5F1E8] text-[#5D5D5D]"
              >
                {tag.display_name || formatCategory(tag.value)}
              </span>
            ))}
          </div>

          {/* TLDR */}
          {post.tldr && (
            <div className="bg-[#F5F1E8] border border-[#E8DED2] rounded-lg px-4 py-3 mb-6">
              <p className="text-[#4D4D4D] text-sm leading-relaxed">{post.tldr}</p>
            </div>
          )}

          {/* Description */}
          <div className="prose max-w-none mb-8">
            <ReactMarkdown
              components={{
                p: ({ children }) => (
                  <p className="mb-4 text-[#4D4D4D] leading-relaxed">{children}</p>
                ),
                ul: ({ children }) => (
                  <ul className="list-disc pl-6 mb-4 space-y-1">{children}</ul>
                ),
                ol: ({ children }) => (
                  <ol className="list-decimal pl-6 mb-4 space-y-1">{children}</ol>
                ),
                li: ({ children }) => (
                  <li className="text-[#4D4D4D]">{children}</li>
                ),
                strong: ({ children }) => (
                  <strong className="font-semibold text-[#3D3D3D]">{children}</strong>
                ),
                a: ({ href, children }) => (
                  <a
                    href={href}
                    className="text-[#8B6D3F] hover:text-[#6D5530] underline"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    {children}
                  </a>
                ),
                h1: ({ children }) => (
                  <h2 className="text-xl font-bold text-[#3D3D3D] mt-6 mb-3">{children}</h2>
                ),
                h2: ({ children }) => (
                  <h3 className="text-lg font-bold text-[#3D3D3D] mt-5 mb-2">{children}</h3>
                ),
                h3: ({ children }) => (
                  <h4 className="text-base font-semibold text-[#3D3D3D] mt-4 mb-2">{children}</h4>
                ),
              }}
            >
              {post.description_markdown || post.description || ""}
            </ReactMarkdown>
          </div>

          {/* Source link CTA */}
          {post.source_url && (
            <div className="border-t border-[#E8DED2] pt-6">
              <a
                href={
                  post.source_url.startsWith("http")
                    ? post.source_url
                    : `https://${post.source_url}`
                }
                target="_blank"
                rel="noopener noreferrer"
                onClick={handleSourceClick}
                className="inline-flex items-center gap-2 px-6 py-3 bg-[#3D3D3D] text-white text-sm font-semibold rounded-full hover:bg-[#2D2D2D] transition-colors"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
                Visit Source
              </a>
            </div>
          )}
        </div>

        {/* Comments */}
        <CommentsSection postId={postId} />
      </section>
    </div>
  );
}
