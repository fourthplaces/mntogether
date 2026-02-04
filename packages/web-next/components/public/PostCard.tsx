"use client";

import { useEffect } from "react";
import { graphqlMutateClient } from "@/lib/graphql/client";
import { TRACK_POST_VIEW, TRACK_POST_CLICK } from "@/lib/graphql/mutations";
import type { Post, PostType, Urgency, CapacityStatus } from "@/lib/types";

interface PostCardProps {
  post: Post;
}

export function PostCard({ post }: PostCardProps) {
  useEffect(() => {
    // Track view on mount
    graphqlMutateClient(TRACK_POST_VIEW, { postId: post.id }).catch(() => {
      // Silently fail - tracking is not critical
    });
  }, [post.id]);

  const handleSourceClick = () => {
    graphqlMutateClient(TRACK_POST_CLICK, { postId: post.id }).catch(() => {
      // Silently fail - tracking is not critical
    });
  };

  const getUrgencyStyles = (urgency?: Urgency) => {
    switch (urgency) {
      case "urgent":
        return {
          bg: "bg-red-50",
          border: "border-red-200",
          text: "text-red-700",
          badge: "bg-red-100 text-red-700",
        };
      case "high":
        return {
          bg: "bg-orange-50",
          border: "border-orange-200",
          text: "text-orange-700",
          badge: "bg-orange-100 text-orange-700",
        };
      case "medium":
        return {
          bg: "bg-amber-50",
          border: "border-amber-200",
          text: "text-amber-700",
          badge: "bg-amber-100 text-amber-700",
        };
      default:
        return {
          bg: "bg-white",
          border: "border-gray-200",
          text: "text-gray-700",
          badge: "bg-gray-100 text-gray-700",
        };
    }
  };

  const getPostTypeStyles = (postType?: PostType) => {
    switch (postType) {
      case "service":
        return { bg: "bg-blue-100", text: "text-blue-700", icon: "\u{1F3E5}", label: "Service" };
      case "opportunity":
        return { bg: "bg-emerald-100", text: "text-emerald-700", icon: "\u{1F91D}", label: "Opportunity" };
      case "business":
        return { bg: "bg-purple-100", text: "text-purple-700", icon: "\u{1F3EA}", label: "Business" };
      case "professional":
        return { bg: "bg-indigo-100", text: "text-indigo-700", icon: "\u{1F464}", label: "Professional" };
      default:
        return { bg: "bg-gray-100", text: "text-gray-700", icon: "\u{1F4CB}", label: "Resource" };
    }
  };

  const getCapacityStyles = (status?: CapacityStatus) => {
    switch (status) {
      case "accepting":
        return { bg: "bg-green-100", text: "text-green-700", label: "Accepting" };
      case "paused":
        return { bg: "bg-yellow-100", text: "text-yellow-700", label: "Paused" };
      case "at_capacity":
        return { bg: "bg-red-100", text: "text-red-700", label: "At Capacity" };
      default:
        return null;
    }
  };

  const formatTimeAgo = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diffInDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));

    if (diffInDays === 0) return "Today";
    if (diffInDays === 1) return "Yesterday";
    if (diffInDays < 7) return `${diffInDays} days ago`;
    if (diffInDays < 30) return `${Math.floor(diffInDays / 7)} weeks ago`;
    return `${Math.floor(diffInDays / 30)} months ago`;
  };

  const urgencyStyles = getUrgencyStyles(post.urgency);
  const postTypeStyles = getPostTypeStyles(post.postType);
  const capacityStyles = getCapacityStyles(post.capacityStatus);

  return (
    <div
      className={`rounded-xl border ${urgencyStyles.border} ${urgencyStyles.bg} p-5 hover:shadow-lg transition-all duration-200 flex flex-col h-full`}
    >
      {/* Header: Post Type + Urgency */}
      <div className="flex items-center justify-between mb-3">
        <span
          className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${postTypeStyles.bg} ${postTypeStyles.text}`}
        >
          <span>{postTypeStyles.icon}</span>
          {postTypeStyles.label}
        </span>
        <div className="flex items-center gap-2">
          {capacityStyles && (
            <span
              className={`px-2 py-0.5 rounded-full text-xs font-medium ${capacityStyles.bg} ${capacityStyles.text}`}
            >
              {capacityStyles.label}
            </span>
          )}
          {post.urgency && post.urgency !== "low" && (
            <span className={`px-2.5 py-1 rounded-full text-xs font-semibold ${urgencyStyles.badge}`}>
              {post.urgency.toUpperCase()}
            </span>
          )}
        </div>
      </div>

      {/* Title */}
      <h3 className="text-lg font-semibold text-gray-900 mb-1 line-clamp-2">{post.title}</h3>

      {/* Organization */}
      <p className="text-sm font-medium text-gray-600 mb-2">{post.organizationName}</p>

      {/* Category + Location */}
      <div className="flex flex-wrap items-center gap-2 text-sm text-gray-500 mb-3">
        {post.category && (
          <span className="inline-flex items-center gap-1 bg-gray-100 px-2 py-0.5 rounded text-xs">
            {post.category}
          </span>
        )}
        {post.location && (
          <span className="inline-flex items-center gap-1">
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
      </div>

      {/* TLDR / Description */}
      {post.tldr ? (
        <p className="text-gray-700 text-sm mb-4 line-clamp-3 flex-grow">{post.tldr}</p>
      ) : (
        <p className="text-gray-700 text-sm mb-4 line-clamp-3 flex-grow">{post.description}</p>
      )}

      {/* Footer: Source Link + Time */}
      <div className="mt-auto pt-3 border-t border-gray-200/60">
        {post.sourceUrl && (
          <div className="mb-2">
            <a
              href={post.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              onClick={handleSourceClick}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 transition-colors"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                />
              </svg>
              Learn More
            </a>
          </div>
        )}
        <p className="text-xs text-gray-400">Posted {formatTimeAgo(post.createdAt)}</p>
      </div>
    </div>
  );
}

// Skeleton loader for loading state
export function PostCardSkeleton() {
  return (
    <div className="rounded-xl border border-gray-200 bg-white p-5 animate-pulse">
      <div className="flex items-center justify-between mb-3">
        <div className="h-6 w-20 bg-gray-200 rounded-full"></div>
        <div className="h-6 w-16 bg-gray-200 rounded-full"></div>
      </div>
      <div className="h-6 w-3/4 bg-gray-200 rounded mb-2"></div>
      <div className="h-4 w-1/2 bg-gray-200 rounded mb-3"></div>
      <div className="flex gap-2 mb-3">
        <div className="h-5 w-16 bg-gray-200 rounded"></div>
        <div className="h-5 w-24 bg-gray-200 rounded"></div>
      </div>
      <div className="space-y-2 mb-4">
        <div className="h-4 w-full bg-gray-200 rounded"></div>
        <div className="h-4 w-5/6 bg-gray-200 rounded"></div>
      </div>
      <div className="pt-3 border-t border-gray-100">
        <div className="flex gap-2">
          <div className="h-8 w-24 bg-gray-200 rounded-lg"></div>
        </div>
      </div>
    </div>
  );
}
