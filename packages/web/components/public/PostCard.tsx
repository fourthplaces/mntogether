"use client";

import { useState } from "react";
import Link from "next/link";
import type { PublicPostResult, PostTypeOption } from "@/lib/restate/types";
import { Badge } from "@/components/ui/Badge";
import { Card } from "@/components/ui/Card";
import { Dialog } from "@/components/ui/Dialog";

function formatCategory(value: string): string {
  return value
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

export function PostCard({ post }: { post: PublicPostResult; postTypes?: PostTypeOption[] }) {
  const [showUrgent, setShowUrgent] = useState(false);
  const postTypeTag = post.tags.find((t) => t.kind === "post_type");
  const displayTags = post.tags.filter((t) => t.kind !== "post_type");
  const urgentNotes = post.urgent_notes ?? [];

  return (
    <>
      <Link href={`/posts/${post.id}`} className="block">
        <Card variant="interactive">
          <div className="flex items-center gap-2 mb-1">
            <h3 className="text-xl font-bold text-text-primary">{post.title}</h3>
            {urgentNotes.length > 0 && (
              <button
                type="button"
                onClick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  setShowUrgent(true);
                }}
                className="shrink-0"
              >
                <Badge variant="danger" size="sm">Urgent</Badge>
              </button>
            )}
          </div>
          {post.location && (
            <p className="text-sm text-text-muted mb-1">{post.location}</p>
          )}
          <p className="text-text-secondary text-[0.95rem] leading-relaxed mb-3">
            {post.summary || post.description}
          </p>
          <div className="flex flex-wrap gap-2">
            {postTypeTag && (
              <Badge
                color={postTypeTag.color || undefined}
                variant="default"
                size="md"
                title={`${postTypeTag.kind}: ${postTypeTag.value}`}
              >
                {postTypeTag.display_name || formatCategory(postTypeTag.value)}
              </Badge>
            )}
            {displayTags.map((tag) => (
              <Badge
                key={tag.value}
                color={tag.color || undefined}
                variant="default"
                size="md"
                title={`${tag.kind}: ${tag.value}`}
              >
                {tag.display_name || formatCategory(tag.value)}
              </Badge>
            ))}
          </div>
        </Card>
      </Link>

      <Dialog
        isOpen={showUrgent}
        onClose={() => setShowUrgent(false)}
        title="Urgent Notes"
        footer={
          <button
            onClick={() => setShowUrgent(false)}
            className="px-4 py-2 text-sm font-medium text-text-secondary hover:text-text-primary"
          >
            Close
          </button>
        }
      >
        <div className="space-y-3 max-h-80 overflow-y-auto">
          {urgentNotes.map((note, i) => (
            <div key={i}>
              {note.cta_text && (
                <p className="text-sm font-semibold text-danger-text">{note.cta_text}</p>
              )}
              <p className="text-sm text-text-body leading-relaxed">{note.content}</p>
            </div>
          ))}
        </div>
      </Dialog>
    </>
  );
}

export function PostCardSkeleton() {
  return (
    <Card variant="default">
      <div className="animate-pulse">
        <div className="h-6 w-3/4 bg-gray-200 rounded mb-2" />
        <div className="h-4 w-1/3 bg-gray-200 rounded mb-2" />
        <div className="h-4 w-full bg-gray-200 rounded mb-1" />
        <div className="h-4 w-5/6 bg-gray-200 rounded mb-3" />
        <div className="h-6 w-20 bg-gray-200 rounded-full" />
      </div>
    </Card>
  );
}
