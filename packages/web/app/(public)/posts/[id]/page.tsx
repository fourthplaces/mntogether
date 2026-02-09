"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useRestateObject, callObject } from "@/lib/restate/client";
import { useEffect } from "react";
import type { PostResult, TagResult } from "@/lib/restate/types";
import CommentsSection from "@/components/public/CommentsSection";

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
      <div className="min-h-screen bg-white">
        <div className="max-w-2xl mx-auto px-4 py-8 animate-pulse">
          <div className="h-4 w-24 bg-gray-200 rounded mb-8" />
          <div className="h-8 w-3/4 bg-gray-200 rounded mb-4" />
          <div className="h-4 w-1/2 bg-gray-200 rounded mb-6" />
          <div className="h-20 w-full bg-gray-100 rounded mb-6" />
          <div className="space-y-3">
            <div className="h-4 w-full bg-gray-200 rounded" />
            <div className="h-4 w-5/6 bg-gray-200 rounded" />
            <div className="h-4 w-4/6 bg-gray-200 rounded" />
          </div>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-white">
        <div className="max-w-2xl mx-auto px-4 py-16 text-center">
          <h1 className="text-xl font-semibold text-gray-900 mb-2">
            Post not found
          </h1>
          <Link
            href="/"
            className="text-blue-600 hover:text-blue-800 text-sm"
          >
            Back to home
          </Link>
        </div>
      </div>
    );
  }

  const tags = post.tags || [];
  const serviceOfferedTags = tags.filter(
    (t: TagResult) => t.kind === "service_offered"
  );

  return (
    <div className="min-h-screen bg-white">
      <div className="max-w-2xl mx-auto px-4 py-8">
        {/* Back link */}
        <Link
          href="/"
          className="inline-flex items-center text-sm text-gray-500 hover:text-gray-700 mb-6"
        >
          &larr; Back
        </Link>

        {/* Title */}
        <h1 className="text-2xl sm:text-3xl font-bold text-gray-900 mb-3">
          {post.title}
        </h1>

        {/* Meta row: location, time, tags */}
        <div className="flex flex-wrap items-center gap-3 text-sm text-gray-500 mb-6">
          {post.location && (
            <span className="inline-flex items-center gap-1">
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
                />
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"
                />
              </svg>
              {post.location}
            </span>
          )}
          <span>Posted {formatTimeAgo(post.created_at)}</span>
        </div>

        {/* TLDR */}
        {post.tldr && (
          <div className="bg-blue-50 border border-blue-100 rounded-lg px-4 py-3 mb-6">
            <p className="text-gray-800 text-sm leading-relaxed">{post.tldr}</p>
          </div>
        )}

        {/* Tags */}
        {serviceOfferedTags.length > 0 && (
          <div className="flex flex-wrap gap-2 mb-6">
            {serviceOfferedTags.map((tag: TagResult) => (
              <span
                key={tag.id}
                className="px-3 py-1 rounded-full text-xs font-medium"
                style={
                  tag.color
                    ? { backgroundColor: tag.color + "15", color: tag.color }
                    : { backgroundColor: "#eff6ff", color: "#2563eb" }
                }
              >
                {tag.display_name || formatCategory(tag.value)}
              </span>
            ))}
          </div>
        )}

        {/* Description */}
        <div className="prose prose-gray max-w-none mb-8">
          <ReactMarkdown
            components={{
              p: ({ children }) => (
                <p className="mb-4 text-gray-700 leading-relaxed">{children}</p>
              ),
              ul: ({ children }) => (
                <ul className="list-disc pl-6 mb-4 space-y-1">{children}</ul>
              ),
              ol: ({ children }) => (
                <ol className="list-decimal pl-6 mb-4 space-y-1">{children}</ol>
              ),
              li: ({ children }) => (
                <li className="text-gray-700">{children}</li>
              ),
              strong: ({ children }) => (
                <strong className="font-semibold">{children}</strong>
              ),
              a: ({ href, children }) => (
                <a
                  href={href}
                  className="text-blue-600 hover:text-blue-800 underline"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  {children}
                </a>
              ),
              h1: ({ children }) => (
                <h2 className="text-xl font-bold text-gray-900 mt-6 mb-3">
                  {children}
                </h2>
              ),
              h2: ({ children }) => (
                <h3 className="text-lg font-bold text-gray-900 mt-5 mb-2">
                  {children}
                </h3>
              ),
              h3: ({ children }) => (
                <h4 className="text-base font-semibold text-gray-800 mt-4 mb-2">
                  {children}
                </h4>
              ),
            }}
          >
            {post.description_markdown || post.description || ""}
          </ReactMarkdown>
        </div>

        {/* Source link CTA */}
        {post.source_url && (
          <div className="border-t border-gray-100 pt-6">
            <a
              href={
                post.source_url.startsWith("http")
                  ? post.source_url
                  : `https://${post.source_url}`
              }
              target="_blank"
              rel="noopener noreferrer"
              onClick={handleSourceClick}
              className="inline-flex items-center gap-2 px-5 py-2.5 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors"
            >
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                />
              </svg>
              Visit Source
            </a>
          </div>
        )}

        {/* Comments */}
        <CommentsSection postId={postId} />
      </div>
    </div>
  );
}
