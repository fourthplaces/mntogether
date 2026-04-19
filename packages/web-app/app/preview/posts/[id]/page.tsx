"use client";

/**
 * Admin-only post preview — /preview/posts/[id]
 *
 * Mirrors /preview/[editionId]: uses an admin-gated GraphQL query
 * (postPreview) that returns UNAUTHENTICATED for non-admins, which
 * surfaces as an "Admin Access Required" banner rather than a generic
 * 404. Renders any post regardless of status so editors can walk a
 * draft (or rejected, or pending) post before publishing.
 */

import Link from "next/link";
import { useParams } from "next/navigation";
import { useCallback } from "react";
import { useQuery } from "urql";

import { PostPreviewQuery } from "@/lib/graphql/public";
import {
  PostDetailView,
  PostDetailSkeleton,
} from "@/components/broadsheet/PostDetailView";

const STATUS_LABELS: Record<string, string> = {
  draft: "Draft",
  pending_approval: "Pending Approval",
  active: "Active",
  filled: "Filled",
  rejected: "Rejected",
  expired: "Expired",
  archived: "Archived",
};

export default function PostPreviewPage() {
  const { id: postId } = useParams<{ id: string }>();

  const [{ data, fetching, error }, reexecute] = useQuery({
    query: PostPreviewQuery,
    variables: { id: postId },
    pause: !postId,
  });

  const handleRefresh = useCallback(() => {
    reexecute({ requestPolicy: "network-only" });
  }, [reexecute]);

  // Surface auth failure as a bespoke message. Anything else falls
  // through to the generic error block below.
  if (error) {
    const isAuthError = error.graphQLErrors?.some(
      (e) =>
        e.extensions?.code === "UNAUTHENTICATED" ||
        e.extensions?.code === "FORBIDDEN"
    );
    if (isAuthError) {
      return (
        <div
          className="broadsheet-page"
          style={{ textAlign: "center", padding: "6rem 1rem" }}
        >
          <h1
            style={{
              fontFamily: "var(--font-feature-deck)",
              fontSize: "1.5rem",
              color: "#fff",
              marginBottom: "0.5rem",
            }}
          >
            Admin Access Required
          </h1>
          <p className="mono-sm" style={{ color: "rgba(255,255,255,0.6)" }}>
            Please log in to the admin app first, then try this link again.
          </p>
        </div>
      );
    }

    return (
      <div
        className="broadsheet-page"
        style={{ textAlign: "center", padding: "6rem 1rem" }}
      >
        <h1
          style={{
            fontFamily: "var(--font-feature-deck)",
            fontSize: "1.5rem",
            color: "#fff",
            marginBottom: "0.5rem",
          }}
        >
          Error Loading Preview
        </h1>
        <p className="mono-sm" style={{ color: "rgba(255,255,255,0.6)" }}>
          {error.message}
        </p>
      </div>
    );
  }

  const post = data?.postPreview;

  if (fetching && !post) return <PostDetailSkeleton />;

  if (!post) {
    return (
      <div
        className="broadsheet-page"
        style={{ textAlign: "center", padding: "6rem 1rem" }}
      >
        <h1
          style={{
            fontFamily: "var(--font-feature-deck)",
            fontSize: "1.5rem",
            color: "#fff",
            marginBottom: "0.5rem",
          }}
        >
          Post Not Found
        </h1>
        <p className="mono-sm" style={{ color: "rgba(255,255,255,0.6)" }}>
          This post may have been deleted or the ID is invalid.
        </p>
      </div>
    );
  }

  const statusLabel = STATUS_LABELS[post.status] ?? post.status;

  // Same visual idiom as the edition preview banner so editors get a
  // consistent "you are in preview mode" signal across routes.
  const banner = (
    <div className="admin-bar">
      <div className="admin-bar__inner">
        <span className="admin-bar__badge">{statusLabel}</span>
        <span>
          {post.status !== "active" ? "PREVIEW — Not Published" : "PREVIEW"}
        </span>
        <button onClick={handleRefresh} className="admin-bar__button">
          {fetching ? "⟳" : "Refresh"}
        </button>
        <Link
          href={`/admin/posts/${postId}`}
          className="admin-bar__link"
        >
          Edit in CMS &rarr;
        </Link>
      </div>
    </div>
  );

  return <PostDetailView post={post} banner={banner} />;
}
