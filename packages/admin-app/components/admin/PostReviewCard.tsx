"use client";

import { useState } from "react";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import { MoreHorizontal } from "lucide-react";

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

const TYPE_VARIANTS: Record<string, "info" | "success" | "spotlight" | "warning" | "secondary"> = {
  story: "info",
  notice: "secondary",
  exchange: "success",
  event: "warning",
  spotlight: "spotlight",
  reference: "info",
};

const URGENCY_VARIANTS: Record<string, "danger" | "warning" | "success" | "secondary"> = {
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

  const tags = post.tags || [];

  return (
    <div className="bg-card border border-border rounded-lg shadow-sm p-4 hover:shadow-md transition-shadow">
      {/* Header */}
      <div className="flex items-start justify-between mb-2">
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-1">
            <Badge variant={TYPE_VARIANTS[post.postType ?? ""] ?? "secondary"}>
              {post.postType || "post"}
            </Badge>
            {post.urgency && (
              <Badge variant={URGENCY_VARIANTS[post.urgency.toLowerCase()] ?? "secondary"}>
                {post.urgency}
              </Badge>
            )}
            {post.category && (
              <Badge variant="secondary">
                {post.category}
              </Badge>
            )}
          </div>
          <Link href={`/admin/posts/${post.id}`} className="text-lg font-semibold text-foreground hover:text-admin-accent transition-colors">
            {post.title}
          </Link>
        </div>

        {/* More menu — replaces hand-rolled dropdown with Radix DropdownMenu */}
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button variant="ghost" size="icon-xs" className="ml-2" />}>
              <MoreHorizontal size={14} strokeWidth={2} />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem render={<Link href={`/admin/posts/${post.id}`} />}>
              Edit Tags
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* Summary */}
      {post.summary && <p className="text-sm text-muted-foreground mb-2">{post.summary}</p>}

      {/* Tags */}
      {tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mb-2">
          {tags.map((tag) => (
            <Badge key={tag.id} variant="secondary">
              <span className="text-muted-foreground">{tag.kind}:</span> {tag.displayName || tag.value}
            </Badge>
          ))}
        </div>
      )}

      {/* Description (collapsed) */}
      <p className={`text-sm text-muted-foreground ${!expanded && "line-clamp-2"}`}>{post.description}</p>

      {/* Expand button */}
      <Button
        variant="link"
        size="xs"
        className="px-0 text-admin-accent hover:text-admin-accent-hover mt-1"
        onClick={() => setExpanded(!expanded)}
      >
        {expanded ? "Show less" : "Show more"}
      </Button>

      {/* Expanded details */}
      {expanded && (
        <div className="mt-3 space-y-3 pt-3 border-t border-border">
          {post.location && (
            <div>
              <span className="font-semibold text-sm text-foreground">Location:</span>{" "}
              <span className="text-sm text-muted-foreground">{post.location}</span>
            </div>
          )}
          {post.sourceUrl && (
            <div>
              <span className="font-semibold text-sm text-foreground">Source:</span>{" "}
              <a
                href={post.sourceUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-admin-accent hover:text-admin-accent-hover break-all"
              >
                {post.sourceUrl}
              </a>
            </div>
          )}
        </div>
      )}

      {/* Actions */}
      {(onApprove || onReject) && (
        <div className="flex gap-2 mt-4 pt-3 border-t border-border">
          {onApprove && (
            <Button
              variant="success"
              className="flex-1"
              onClick={() => onApprove(post.id)}
              loading={isApproving}
            >
              Approve
            </Button>
          )}
          {onReject && (
            <Button
              variant="destructive"
              className="flex-1"
              onClick={() => onReject(post.id)}
              loading={isRejecting}
            >
              Reject
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
