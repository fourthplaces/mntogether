"use client";

import { useCallback } from "react";
import { useParams } from "next/navigation";
import { useQuery } from "urql";
import { EditionPreviewQuery } from "@/lib/graphql/broadsheet";
import { BroadsheetRenderer, SiteFooter } from "@/components/broadsheet";

const STATUS_LABELS: Record<string, string> = {
  draft: "Draft",
  in_review: "In Review",
  approved: "Approved",
  published: "Published",
  archived: "Archived",
};

export default function PreviewPage() {
  const { editionId } = useParams<{ editionId: string }>();

  const [{ data, fetching, error }, reexecuteQuery] = useQuery({
    query: EditionPreviewQuery,
    variables: { editionId },
    pause: !editionId,
  });

  const handleRefresh = useCallback(() => {
    reexecuteQuery({ requestPolicy: "network-only" });
  }, [reexecuteQuery]);

  const edition = data?.editionPreview;

  // Auth error — the API returns UNAUTHENTICATED if no valid admin cookie
  if (error) {
    const isAuthError = error.graphQLErrors?.some(
      (e) =>
        e.extensions?.code === "UNAUTHENTICATED" ||
        e.extensions?.code === "FORBIDDEN"
    );

    if (isAuthError) {
      return (
        <div className="broadsheet-page" style={{ textAlign: "center", padding: "6rem 1rem" }}>
          <h1 style={{ fontFamily: "var(--font-feature-deck)", fontSize: "1.5rem", color: "#fff", marginBottom: "0.5rem" }}>
            Admin Access Required
          </h1>
          <p className="mono-sm" style={{ color: "rgba(255,255,255,0.6)" }}>
            Please log in to the admin app first, then try this link again.
          </p>
        </div>
      );
    }

    return (
      <div className="broadsheet-page" style={{ textAlign: "center", padding: "6rem 1rem" }}>
        <h1 style={{ fontFamily: "var(--font-feature-deck)", fontSize: "1.5rem", color: "#fff", marginBottom: "0.5rem" }}>
          Error Loading Preview
        </h1>
        <p className="mono-sm" style={{ color: "rgba(255,255,255,0.6)" }}>
          {error.message}
        </p>
      </div>
    );
  }

  // Loading state (initial only)
  if (fetching && !edition) {
    return (
      <div className="broadsheet-page" style={{ textAlign: "center", padding: "6rem 1rem" }}>
        <p className="mono-sm" style={{ color: "rgba(255,255,255,0.5)" }}>
          Loading preview...
        </p>
      </div>
    );
  }

  // No edition found
  if (!edition) {
    return (
      <div className="broadsheet-page" style={{ textAlign: "center", padding: "6rem 1rem" }}>
        <h1 style={{ fontFamily: "var(--font-feature-deck)", fontSize: "1.5rem", color: "#fff", marginBottom: "0.5rem" }}>
          Edition Not Found
        </h1>
        <p className="mono-sm" style={{ color: "rgba(255,255,255,0.6)" }}>
          This edition may have been deleted or the ID is invalid.
        </p>
      </div>
    );
  }

  const statusLabel = STATUS_LABELS[edition.status] ?? edition.status;

  return (
    <div className="broadsheet-page">
      {/* Preview banner */}
      <div className="admin-bar">
        <div className="admin-bar__inner">
          <span className="admin-bar__badge">
            {statusLabel}
          </span>
          <span>
            {edition.status !== "published" ? "PREVIEW — Not Published" : "PREVIEW"}
          </span>
          <button
            onClick={handleRefresh}
            className="admin-bar__button"
          >
            {fetching ? "⟳" : "Refresh"}
          </button>
        </div>
      </div>
      <BroadsheetRenderer edition={edition} />
      <SiteFooter />
    </div>
  );
}
