"use client";

import { useState, useRef, useEffect } from "react";
import Link from "next/link";
import type { PostResult } from "@/lib/restate/types";

interface PostReviewCardProps {
  post: PostResult;
  onApprove?: (id: string) => void;
  onReject?: (id: string, reason?: string) => void;
  isApproving?: boolean;
  isRejecting?: boolean;
}

export function PostReviewCard({
  post,
  onApprove,
  onReject,
  isApproving,
  isRejecting,
}: PostReviewCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [showRejectModal, setShowRejectModal] = useState(false);
  const [rejectReason, setRejectReason] = useState("");
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const getTypeColor = (type?: string | null) => {
    switch (type) {
      case "service":
        return "bg-blue-100 text-blue-800";
      case "opportunity":
        return "bg-green-100 text-green-800";
      case "business":
        return "bg-purple-100 text-purple-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const getUrgencyColor = (urgency?: string | null) => {
    switch (urgency?.toLowerCase()) {
      case "urgent":
        return "bg-red-100 text-red-800";
      case "high":
        return "bg-orange-100 text-orange-800";
      case "medium":
        return "bg-yellow-100 text-yellow-800";
      case "low":
        return "bg-green-100 text-green-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const handleReject = () => {
    onReject?.(post.id, rejectReason);
    setShowRejectModal(false);
    setRejectReason("");
  };

  const tags = post.tags || [];

  return (
    <>
      <div className="bg-white border border-stone-200 rounded-lg shadow-sm p-4 hover:shadow-md transition-shadow">
        {/* Header */}
        <div className="flex items-start justify-between mb-2">
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className={`px-2 py-1 text-xs font-medium rounded ${getTypeColor(post.post_type)}`}>
                {post.post_type || "post"}
              </span>
              {post.urgency && (
                <span
                  className={`px-2 py-1 text-xs font-medium rounded ${getUrgencyColor(post.urgency)}`}
                >
                  {post.urgency}
                </span>
              )}
              {post.category && (
                <span className="px-2 py-1 text-xs bg-stone-100 text-stone-700 rounded">
                  {post.category}
                </span>
              )}
            </div>
            <Link href={`/admin/posts/${post.id}`} className="text-lg font-semibold text-stone-900 hover:text-amber-700 transition-colors">
              {post.title}
            </Link>
          </div>

          {/* More menu */}
          <div className="relative ml-2" ref={menuRef}>
            <button
              onClick={() => setMenuOpen(!menuOpen)}
              className="p-1.5 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded"
            >
              {"\u22EF"}
            </button>
            {menuOpen && (
              <div className="absolute right-0 mt-1 w-36 bg-white rounded-lg shadow-lg border border-stone-200 py-1 z-10">
                <Link
                  href={`/admin/posts/${post.id}`}
                  className="block w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50"
                >
                  Edit Tags
                </Link>
              </div>
            )}
          </div>
        </div>

        {/* TLDR */}
        {post.tldr && <p className="text-sm text-stone-700 italic mb-2">&quot;{post.tldr}&quot;</p>}

        {/* Tags */}
        {tags.length > 0 && (
          <div className="flex flex-wrap gap-1.5 mb-2">
            {tags.map((tag) => (
              <span
                key={tag.id}
                className="px-2 py-0.5 text-xs rounded-full bg-stone-100 text-stone-600"
              >
                <span className="text-stone-400">{tag.kind}:</span> {tag.display_name || tag.value}
              </span>
            ))}
          </div>
        )}

        {/* Description (collapsed) */}
        <p className={`text-sm text-stone-600 ${!expanded && "line-clamp-2"}`}>{post.description}</p>

        {/* Expand button */}
        <button
          onClick={() => setExpanded(!expanded)}
          className="text-xs text-amber-600 hover:text-amber-800 mt-1"
        >
          {expanded ? "Show less" : "Show more"}
        </button>

        {/* Expanded details */}
        {expanded && (
          <div className="mt-3 space-y-3 pt-3 border-t border-stone-200">
            {/* Location */}
            {post.location && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Location:</span>{" "}
                <span className="text-sm text-stone-600">{post.location}</span>
              </div>
            )}

            {/* Source URL */}
            {post.source_url && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Source:</span>{" "}
                <a
                  href={post.source_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-amber-600 hover:text-amber-800 break-all"
                >
                  {post.source_url}
                </a>
              </div>
            )}
          </div>
        )}

        {/* Actions */}
        {(onApprove || onReject) && (
          <div className="flex gap-2 mt-4 pt-3 border-t border-stone-200">
            {onApprove && (
              <button
                onClick={() => onApprove(post.id)}
                disabled={isApproving}
                className="flex-1 px-4 py-2 bg-emerald-400 text-white rounded hover:bg-emerald-500 transition-colors font-medium disabled:opacity-50"
              >
                {isApproving ? "..." : "Approve"}
              </button>
            )}
            {onReject && (
              <button
                onClick={() => setShowRejectModal(true)}
                disabled={isRejecting}
                className="flex-1 px-4 py-2 bg-rose-400 text-white rounded hover:bg-rose-500 transition-colors font-medium disabled:opacity-50"
              >
                {isRejecting ? "..." : "Reject"}
              </button>
            )}
          </div>
        )}
      </div>

      {/* Reject Modal */}
      {showRejectModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg p-6 max-w-md w-full">
            <h3 className="text-lg font-semibold text-stone-900 mb-4">Reject Post</h3>
            <p className="text-sm text-stone-600 mb-4">
              Are you sure you want to reject &quot;{post.title}&quot;? You can optionally provide a
              reason.
            </p>
            <textarea
              value={rejectReason}
              onChange={(e) => setRejectReason(e.target.value)}
              placeholder="Reason for rejection (optional)"
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500 mb-4"
              rows={3}
            />
            <div className="flex gap-2">
              <button
                onClick={handleReject}
                className="flex-1 px-4 py-2 bg-rose-400 text-white rounded hover:bg-rose-500"
              >
                Reject
              </button>
              <button
                onClick={() => {
                  setShowRejectModal(false);
                  setRejectReason("");
                }}
                className="flex-1 px-4 py-2 bg-stone-200 text-stone-700 rounded hover:bg-stone-300"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
