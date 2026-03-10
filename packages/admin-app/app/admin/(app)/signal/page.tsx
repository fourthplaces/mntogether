"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { SignalPostsQuery, RejectPostMutation } from "@/lib/graphql/posts";
import { CountiesQuery } from "@/lib/graphql/editions";

// ─── Types ──────────────────────────────────────────────────────────────────

type PostTypeFilter = "" | "story" | "notice" | "exchange" | "event" | "spotlight" | "reference";

const POST_TYPE_OPTIONS: { value: PostTypeFilter; label: string }[] = [
  { value: "", label: "All Types" },
  { value: "story", label: "Story" },
  { value: "notice", label: "Notice" },
  { value: "exchange", label: "Exchange" },
  { value: "event", label: "Event" },
  { value: "spotlight", label: "Spotlight" },
  { value: "reference", label: "Reference" },
];

const TYPE_BADGE_STYLES: Record<string, string> = {
  story: "bg-indigo-100 text-indigo-800",
  notice: "bg-amber-100 text-amber-800",
  exchange: "bg-blue-100 text-blue-800",
  event: "bg-green-100 text-green-800",
  spotlight: "bg-purple-100 text-purple-800",
  reference: "bg-stone-100 text-stone-700",
};

const WEIGHT_BADGE_STYLES: Record<string, string> = {
  heavy: "bg-red-100 text-red-700",
  medium: "bg-yellow-100 text-yellow-700",
  light: "bg-sky-100 text-sky-700",
};

// ─── Helpers ────────────────────────────────────────────────────────────────

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 7) return `${diffDay}d ago`;
  return new Date(dateStr).toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

// ─── Component ──────────────────────────────────────────────────────────────

export default function SignalPage() {
  const router = useRouter();
  const [countyId, setCountyId] = useState("");
  const [postType, setPostType] = useState<PostTypeFilter>("");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchInput, setSearchInput] = useState("");
  const [showRejected, setShowRejected] = useState(false);
  const [rejectingId, setRejectingId] = useState<string | null>(null);

  const pagination = useOffsetPagination({ pageSize: 25 });

  // Reset pagination when filters change
  useEffect(() => {
    pagination.reset();
  }, [countyId, postType, searchQuery, showRejected]);

  // ─── Queries ──────────────────────────────────────────────────────

  const [{ data: countiesData }] = useQuery({ query: CountiesQuery });
  const counties = countiesData?.counties || [];

  const isStatewide = countyId === "__statewide__";
  const [{ data, fetching, error }] = useQuery({
    query: SignalPostsQuery,
    variables: {
      status: showRejected ? "rejected" : "active",
      countyId: countyId && !isStatewide ? countyId : null,
      statewideOnly: isStatewide || null,
      postType: postType || null,
      search: searchQuery || null,
      limit: pagination.variables.first,
      offset: pagination.variables.offset,
    },
  });

  const [, rejectPost] = useMutation(RejectPostMutation);

  const posts = data?.posts?.posts || [];
  const totalCount = data?.posts?.totalCount || 0;
  const hasNextPage = data?.posts?.hasNextPage || false;
  const pageInfo = pagination.buildPageInfo(hasNextPage);

  // ─── Actions ──────────────────────────────────────────────────────

  const handleReject = async (postId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm("Reject this post? It will be removed from broadsheet eligibility.")) return;
    setRejectingId(postId);
    try {
      await rejectPost(
        { id: postId, reason: "Rejected by editor from Signal view" },
        { additionalTypenames: ["Post", "PostConnection"] }
      );
    } catch (err) {
      console.error("Failed to reject post:", err);
    } finally {
      setRejectingId(null);
    }
  };

  // ─── Render ───────────────────────────────────────────────────────

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-4">
          <h1 className="text-2xl font-bold text-foreground">Signal</h1>
          <p className="text-muted-foreground text-sm mt-0.5">
            Posts ingested from Root Signal &middot; {totalCount.toLocaleString()} posts
          </p>
        </div>

        {/* Filters row */}
        <div className="flex flex-wrap gap-3 mb-4 items-center">
          {/* County dropdown (primary filter) */}
          <select
            value={countyId}
            onChange={(e) => setCountyId(e.target.value)}
            className="px-3 py-2 border border-border rounded-lg text-sm bg-background focus:outline-none focus:ring-2 focus:ring-ring w-52"
          >
            <option value="">All Counties</option>
            <option value="__statewide__">Statewide</option>
            {counties
              .slice()
              .sort((a, b) => a.name.localeCompare(b.name))
              .map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
          </select>

          {/* Type dropdown */}
          <select
            value={postType}
            onChange={(e) => setPostType(e.target.value as PostTypeFilter)}
            className="px-3 py-2 border border-border rounded-lg text-sm bg-background focus:outline-none focus:ring-2 focus:ring-ring w-36"
          >
            {POST_TYPE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>

          {/* Search */}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              setSearchQuery(searchInput);
            }}
            className="flex gap-2 flex-1 min-w-[200px]"
          >
            <input
              type="text"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              placeholder="Search posts..."
              className="flex-1 px-3 py-2 border border-border rounded-lg text-sm bg-background focus:outline-none focus:ring-2 focus:ring-ring"
            />
            {searchQuery && (
              <button
                type="button"
                onClick={() => {
                  setSearchInput("");
                  setSearchQuery("");
                }}
                className="px-3 py-2 text-sm text-muted-foreground hover:text-foreground"
              >
                Clear
              </button>
            )}
          </form>

          {/* Show rejected toggle */}
          <label className="flex items-center gap-2 text-sm text-muted-foreground cursor-pointer select-none">
            <input
              type="checkbox"
              checked={showRejected}
              onChange={(e) => setShowRejected(e.target.checked)}
              className="rounded border-border"
            />
            Show rejected
          </label>
        </div>

        {/* Active filter pills */}
        {(countyId || postType || searchQuery) && (
          <div className="flex gap-2 flex-wrap mb-4">
            {countyId && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-accent text-accent-foreground rounded-full text-xs font-medium">
                County: {countyId === "__statewide__" ? "Statewide" : counties.find((c) => c.id === countyId)?.name || countyId}
                <button onClick={() => setCountyId("")} className="hover:text-foreground">&times;</button>
              </span>
            )}
            {postType && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-accent text-accent-foreground rounded-full text-xs font-medium">
                Type: <span className="capitalize">{postType}</span>
                <button onClick={() => setPostType("")} className="hover:text-foreground">&times;</button>
              </span>
            )}
            {searchQuery && (
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-accent text-accent-foreground rounded-full text-xs font-medium">
                Search: {searchQuery}
                <button onClick={() => { setSearchInput(""); setSearchQuery(""); }} className="hover:text-foreground">&times;</button>
              </span>
            )}
          </div>
        )}

        {/* Error */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-4 text-sm">
            Error: {error.message}
          </div>
        )}

        {/* Loading */}
        {fetching && posts.length === 0 && (
          <AdminLoader label="Loading signal posts..." />
        )}

        {/* Table */}
        {!fetching && !error && posts.length === 0 ? (
          <div className="bg-card border border-border rounded-lg p-12 text-center">
            <h3 className="text-lg font-semibold text-foreground mb-1">No posts found</h3>
            <p className="text-muted-foreground text-sm">
              {searchQuery || countyId || postType
                ? "Try adjusting your filters."
                : showRejected
                ? "No rejected signal posts."
                : "No active signal posts in the system."}
            </p>
          </div>
        ) : posts.length > 0 && (
          <>
            <div className="bg-card rounded-lg shadow-sm border border-border overflow-hidden">
              <table className="min-w-full divide-y divide-border">
                <thead className="bg-secondary">
                  <tr>
                    <th className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider">
                      Title
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider w-24">
                      Type
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider w-24">
                      Weight
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider">
                      Source
                    </th>
                    <th className="px-4 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider w-24">
                      Date
                    </th>
                    <th className="w-12" />
                  </tr>
                </thead>
                <tbody className="divide-y divide-border">
                  {posts.map((post) => (
                    <tr
                      key={post.id}
                      onClick={() => router.push(`/admin/posts/${post.id}`)}
                      className="hover:bg-secondary cursor-pointer transition-colors"
                    >
                      <td className="px-6 py-3">
                        <div className="font-medium text-foreground text-sm truncate max-w-md">
                          {post.title}
                        </div>
                        {post.location && (
                          <div className="text-xs text-muted-foreground truncate max-w-md mt-0.5">
                            {post.location}
                          </div>
                        )}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        {post.postType && (
                          <span
                            className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                              TYPE_BADGE_STYLES[post.postType] || "bg-secondary text-muted-foreground"
                            }`}
                          >
                            {post.postType}
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap">
                        {post.weight && (
                          <span
                            className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                              WEIGHT_BADGE_STYLES[post.weight] || "bg-secondary text-muted-foreground"
                            }`}
                          >
                            {post.weight}
                          </span>
                        )}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-muted-foreground truncate max-w-[160px]">
                        {post.organizationName || "—"}
                      </td>
                      <td className="px-4 py-3 whitespace-nowrap text-sm text-muted-foreground">
                        {timeAgo(post.createdAt)}
                      </td>
                      <td className="px-3 py-3 whitespace-nowrap">
                        {!showRejected && (
                          <button
                            onClick={(e) => handleReject(post.id, e)}
                            disabled={rejectingId === post.id}
                            className="p-1 text-muted-foreground hover:text-red-600 rounded disabled:opacity-50"
                            title="Reject post"
                          >
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M6 18L18 6M6 6l12 12" />
                            </svg>
                          </button>
                        )}
                        {post.sourceUrl && (
                          <a
                            href={post.sourceUrl}
                            target="_blank"
                            rel="noopener noreferrer"
                            onClick={(e) => e.stopPropagation()}
                            className="p-1 text-muted-foreground hover:text-foreground rounded inline-block"
                            title="View source"
                          >
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M13.5 6H5.25A2.25 2.25 0 003 8.25v10.5A2.25 2.25 0 005.25 21h10.5A2.25 2.25 0 0018 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25" />
                            </svg>
                          </a>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            {/* Pagination */}
            <div className="mt-4">
              <PaginationControls
                pageInfo={pageInfo}
                totalCount={totalCount}
                currentPage={pagination.currentPage}
                pageSize={pagination.pageSize}
                onNextPage={pagination.goToNextPage}
                onPreviousPage={pagination.goToPreviousPage}
                loading={fetching}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
