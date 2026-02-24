"use client";

import { useState } from "react";
import Link from "next/link";
import type { PublicPostsQuery } from "@/lib/graphql/public";
import type { ResultOf } from "@graphql-typed-document-node/core";

type PublicPostsResult = ResultOf<typeof PublicPostsQuery>;
type PublicPost = PublicPostsResult["publicPosts"]["posts"][number];

function formatCategory(value: string): string {
  return value
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

export function PostCard({ post }: { post: PublicPost }) {
  const [showUrgent, setShowUrgent] = useState(false);
  const postTypeTag = post.tags.find((t) => t.kind === "post_type");
  const displayTags = post.tags.filter((t) => t.kind !== "post_type");
  const urgentNotes = post.urgentNotes ?? [];

  return (
    <>
      <Link
        href={`/posts/${post.id}`}
        className="bg-surface-raised p-6 border border-border hover:border-border-strong block"
      >
        {post.organizationName && (
          <p className="text-xs font-medium text-text-muted uppercase tracking-wide mb-0.5">
            {post.organizationId ? (
              <Link
                href={`/organizations/${post.organizationId}`}
                onClick={(e) => e.stopPropagation()}
                className="hover:text-text-primary"
              >
                {post.organizationName}
              </Link>
            ) : (
              post.organizationName
            )}
          </p>
        )}
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
              className="px-2.5 py-0.5 text-xs font-medium bg-red-100 text-red-800 shrink-0 hover:bg-red-200"
            >
              Urgent
            </button>
          )}
        </div>
        {(post.location || post.distanceMiles != null) && (
          <p className="text-sm text-text-muted mb-1">
            {post.location}
            {post.distanceMiles != null && (
              <span className="ml-2 font-medium">
                {post.distanceMiles < 1
                  ? "< 1 mi"
                  : `${Math.round(post.distanceMiles)} mi`}
              </span>
            )}
          </p>
        )}
        <p className="text-text-secondary text-base leading-relaxed mb-3">
          {post.summary || post.description}
        </p>
        <div className="flex flex-wrap gap-2">
          {postTypeTag && (
            <span
              title={`${postTypeTag.kind}: ${postTypeTag.value}`}
              className={`px-3 py-1 text-xs font-medium ${!postTypeTag.color ? "bg-surface-muted text-text-secondary" : ""}`}
              style={postTypeTag.color ? { backgroundColor: postTypeTag.color + "20", color: postTypeTag.color } : undefined}
            >
              {postTypeTag.displayName || formatCategory(postTypeTag.value)}
            </span>
          )}
          {displayTags.map((tag) => (
            <span
              key={tag.value}
              title={`${tag.kind}: ${tag.value}`}
              className={`px-3 py-1 text-xs font-medium ${!tag.color ? "bg-surface-muted text-text-secondary" : ""}`}
              style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
            >
              {tag.displayName || formatCategory(tag.value)}
            </span>
          ))}
        </div>
      </Link>

      {/* Urgent Notes Dialog */}
      {showUrgent && (
        <>
          <div
            className="fixed inset-0 z-40 bg-black/30"
            onClick={() => setShowUrgent(false)}
          />
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div className="bg-surface-raised border border-border w-full max-w-md">
              <div className="flex items-center justify-between px-5 py-4 border-b border-border">
                <div className="flex items-center gap-2">
                  <span className="px-2.5 py-0.5 text-xs font-medium bg-red-100 text-red-800">
                    Urgent
                  </span>
                  <h2 className="text-lg font-semibold text-text-primary">Notes</h2>
                </div>
                <button
                  onClick={() => setShowUrgent(false)}
                  className="text-text-muted hover:text-text-primary text-xl leading-none"
                >
                  &times;
                </button>
              </div>
              <div className="p-5 space-y-3 max-h-80 overflow-y-auto">
                {urgentNotes.map((note, i) => (
                  <div key={i}>
                    {note.ctaText && (
                      <p className="text-sm font-semibold text-red-900">{note.ctaText}</p>
                    )}
                    <p className="text-sm text-text-body leading-relaxed">{note.content}</p>
                  </div>
                ))}
              </div>
              <div className="px-5 py-3 border-t border-border flex justify-end">
                <button
                  onClick={() => setShowUrgent(false)}
                  className="px-4 py-2 text-sm font-medium text-text-secondary hover:text-text-primary"
                >
                  Close
                </button>
              </div>
            </div>
          </div>
        </>
      )}
    </>
  );
}

export function PostCardSkeleton() {
  return (
    <div className="bg-surface-raised border border-border p-6">
      <div className="animate-pulse">
        <div className="h-6 w-3/4 bg-border rounded mb-2" />
        <div className="h-4 w-1/3 bg-border rounded mb-2" />
        <div className="h-4 w-full bg-border rounded mb-1" />
        <div className="h-4 w-5/6 bg-border rounded mb-3" />
        <div className="h-6 w-20 bg-border" />
      </div>
    </div>
  );
}
