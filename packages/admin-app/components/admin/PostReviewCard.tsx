"use client";

import { useState, useRef, useEffect } from "react";
import Link from "next/link";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";

interface PostReviewCardPost {
  id: string;
  title: string;
  description: string;
  summary?: string | null;
  postType?: string | null;
  category?: string | null;
  urgency?: string | null;
  location?: string | null;
  sourceUrl?: string | null;
  submissionType?: string | null;
  relevanceScore?: number | null;
  distanceMiles?: number | null;
  tags: Array<{
    id: string;
    kind: string;
    value: string;
    displayName?: string | null;
    color?: string | null;
  }>;
}

interface PostReviewCardProps {
  post: PostReviewCardPost;
  onApprove?: (id: string) => void;
  onReject?: (id: string, reason?: string) => void;
  isApproving?: boolean;
  isRejecting?: boolean;
}

const TYPE_VARIANTS: Record<string, "info" | "success" | "business" | "default"> = {
  service: "info",
  opportunity: "success",
  business: "business",
};

const URGENCY_VARIANTS: Record<string, "danger" | "warning" | "success" | "default"> = {
  urgent: "danger",
  high: "warning",
  medium: "warning",
  low: "success",
};

export function PostReviewCard({
  post,
  onApprove,
  onReject,
  isApproving,
  isRejecting,
}: PostReviewCardProps) {
  const [expanded, setExpanded] = useState(false);
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

  const getScoreColor = (score: number) => {
    if (score >= 8) return "bg-green-100 text-green-800";
    if (score >= 5) return "bg-amber-100 text-amber-800";
    return "bg-red-100 text-red-800";
  };

  const tags = post.tags || [];

  return (
    <>
      <div className="bg-white border border-stone-200 rounded-lg shadow-sm p-4 hover:shadow-md transition-shadow">
        {/* Header */}
        <div className="flex items-start justify-between mb-2">
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <Badge
                variant={TYPE_VARIANTS[post.postType ?? ""] ?? "default"}
                pill={false}
              >
                {post.postType || "post"}
              </Badge>
              {post.urgency && (
                <Badge
                  variant={URGENCY_VARIANTS[post.urgency.toLowerCase()] ?? "default"}
                  pill={false}
                >
                  {post.urgency}
                </Badge>
              )}
              {post.category && (
                <Badge variant="default" pill={false}>
                  {post.category}
                </Badge>
              )}
              {post.relevanceScore != null && (
                <span className={`px-2 py-1 text-xs font-bold rounded ${getScoreColor(post.relevanceScore)}`}>
                  {post.relevanceScore}/10
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

        {/* Summary */}
        {post.summary && <p className="text-sm text-stone-600 mb-2">{post.summary}</p>}

        {/* Tags */}
        {tags.length > 0 && (
          <div className="flex flex-wrap gap-1.5 mb-2">
            {tags.map((tag) => (
              <Badge key={tag.id} variant="default" size="sm">
                <span className="text-stone-400">{tag.kind}:</span> {tag.displayName || tag.value}
              </Badge>
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
            {post.location && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Location:</span>{" "}
                <span className="text-sm text-stone-600">{post.location}</span>
              </div>
            )}
            {post.sourceUrl && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Source:</span>{" "}
                <a
                  href={post.sourceUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-amber-600 hover:text-amber-800 break-all"
                >
                  {post.sourceUrl}
                </a>
              </div>
            )}
          </div>
        )}

        {/* Actions */}
        {(onApprove || onReject) && (
          <div className="flex gap-2 mt-4 pt-3 border-t border-stone-200">
            {onApprove && (
              <Button
                variant="success"
                size="md"
                className="flex-1"
                onClick={() => onApprove(post.id)}
                disabled={isApproving}
              >
                {isApproving ? "..." : "Approve"}
              </Button>
            )}
            {onReject && (
              <Button
                variant="danger"
                size="md"
                className="flex-1"
                onClick={() => onReject(post.id)}
                disabled={isRejecting}
              >
                {isRejecting ? "..." : "Reject"}
              </Button>
            )}
          </div>
        )}
      </div>
    </>
  );
}
