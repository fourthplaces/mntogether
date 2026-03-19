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
      <div className="post-card">
        {post.organizationName && (
          <p className="org-label" style={{ marginBottom: "0.125rem" }}>
            {post.organizationId ? (
              <Link
                href={`/organizations/${post.organizationId}`}
                className="z-above"
                style={{ display: "inline" }}
              >
                {post.organizationName}
              </Link>
            ) : (
              post.organizationName
            )}
          </p>
        )}
        {/* Stretched link — covers the whole card, sits below z-10 interactive elements */}
        <Link
          href={`/posts/${post.id}`}
          className="post-card-stretched-link"
          aria-label={post.title}
        />
        <div className="post-card-header">
          <h3 className="card-title">{post.title}</h3>
          {urgentNotes.length > 0 && (
            <button
              type="button"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setShowUrgent(true);
              }}
              className="badge-urgent badge-urgent--interactive"
            >
              Urgent
            </button>
          )}
        </div>
        {(post.location || post.distanceMiles != null) && (
          <p className="post-card-location">
            {post.location}
            {post.distanceMiles != null && (
              <span className="post-card-distance">
                {post.distanceMiles < 1
                  ? "< 1 mi"
                  : `${Math.round(post.distanceMiles)} mi`}
              </span>
            )}
          </p>
        )}
        <p className="post-card-summary">
          {post.bodyLight || post.bodyRaw}
        </p>
        <div className="post-card-tags">
          {postTypeTag && (
            <span
              title={`${postTypeTag.kind}: ${postTypeTag.value}`}
              className={`tag ${!postTypeTag.color ? "tag--muted" : ""}`}
              style={postTypeTag.color ? { backgroundColor: postTypeTag.color + "20", color: postTypeTag.color } : undefined}
            >
              {postTypeTag.displayName || formatCategory(postTypeTag.value)}
            </span>
          )}
          {displayTags.map((tag) => (
            <span
              key={tag.value}
              title={`${tag.kind}: ${tag.value}`}
              className={`tag ${!tag.color ? "tag--muted" : ""}`}
              style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
            >
              {tag.displayName || formatCategory(tag.value)}
            </span>
          ))}
        </div>
      </div>

      {/* Urgent Notes Dialog */}
      {showUrgent && (
        <>
          <div
            className="backdrop--light"
            onClick={() => setShowUrgent(false)}
          />
          <div className="dialog">
            <div className="dialog-content">
              <div className="dialog-header">
                <div className="dialog-header-title">
                  <span className="badge-urgent">
                    Urgent
                  </span>
                  <h2 className="group-title--semi">Notes</h2>
                </div>
                <button
                  onClick={() => setShowUrgent(false)}
                  className="dialog-close"
                >
                  &times;
                </button>
              </div>
              <div className="dialog-body">
                {urgentNotes.map((note, i) => (
                  <div key={i}>
                    {note.ctaText && (
                      <p className="urgent-cta">{note.ctaText}</p>
                    )}
                    <p className="text-body" style={{ fontSize: "0.875rem" }}>{note.content}</p>
                  </div>
                ))}
              </div>
              <div className="dialog-footer">
                <button
                  onClick={() => setShowUrgent(false)}
                  className="btn-text"
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
    <div className="card">
      <div className="skeleton">
        <div className="skeleton-line" style={{ height: "1.5rem", width: "75%", marginBottom: "0.5rem" }} />
        <div className="skeleton-line" style={{ height: "1rem", width: "33%", marginBottom: "0.5rem" }} />
        <div className="skeleton-line" style={{ height: "1rem", width: "100%", marginBottom: "0.25rem" }} />
        <div className="skeleton-line" style={{ height: "1rem", width: "83%", marginBottom: "0.75rem" }} />
        <div className="skeleton-line" style={{ height: "1.5rem", width: "5rem" }} />
      </div>
    </div>
  );
}
