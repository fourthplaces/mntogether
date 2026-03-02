"use client";

import { useState, useEffect } from "react";
import { useQuery, useMutation } from "urql";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { PostReviewCard } from "@/components/admin/PostReviewCard";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  PostStatsQuery,
  PostsListQuery,
  ApprovePostMutation,
  RejectPostMutation,
} from "@/lib/graphql/posts";

type PostTypeFilter = "all" | "story" | "notice" | "exchange" | "event" | "spotlight" | "reference";
type StatusFilter = "pending_approval" | "active" | "rejected";

export default function PostsPage() {
  const [selectedStatus, setSelectedStatus] = useState<StatusFilter>("pending_approval");
  const [selectedType, setSelectedType] = useState<PostTypeFilter>("all");
  const [searchInput, setSearchInput] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [zipInput, setZipInput] = useState("");
  const [zipCode, setZipCode] = useState("");
  const [radiusMiles, setRadiusMiles] = useState<number>(25);
  const [approvingId, setApprovingId] = useState<string | null>(null);
  const [rejectingId, setRejectingId] = useState<string | null>(null);

  const pagination = useOffsetPagination({ pageSize: 10 });

  // Reset pagination when filters change
  useEffect(() => {
    pagination.reset();
  }, [selectedStatus, selectedType, searchQuery, zipCode, radiusMiles]);

  // Fetch stats
  const [{ data: statsData }] = useQuery({
    query: PostStatsQuery,
    variables: { status: selectedStatus },
  });

  // Fetch posts with offset pagination and filters
  const [{ data, fetching: isLoading, error }] = useQuery({
    query: PostsListQuery,
    variables: {
      status: selectedStatus,
      postType: selectedType === "all" ? null : selectedType,
      search: searchQuery || null,
      zipCode: zipCode || null,
      radiusMiles: zipCode ? radiusMiles : null,
      limit: pagination.variables.first,
      offset: pagination.variables.offset,
    },
  });

  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, rejectPost] = useMutation(RejectPostMutation);

  const handleApprove = async (postId: string) => {
    setApprovingId(postId);
    try {
      await approvePost({ id: postId }, { additionalTypenames: ["Post", "PostConnection", "PostStats"] });
    } catch (err) {
      console.error("Failed to approve post:", err);
    } finally {
      setApprovingId(null);
    }
  };

  const handleReject = async (postId: string, reason?: string) => {
    setRejectingId(postId);
    try {
      await rejectPost(
        { id: postId, reason: reason || "Rejected by admin" },
        { additionalTypenames: ["Post", "PostConnection", "PostStats"] }
      );
    } catch (err) {
      console.error("Failed to reject post:", err);
    } finally {
      setRejectingId(null);
    }
  };

  const posts = data?.posts?.posts || [];
  const totalCount = data?.posts?.totalCount || 0;
  const hasNextPage = data?.posts?.hasNextPage || false;
  const pageInfo = pagination.buildPageInfo(hasNextPage);

  const stats = {
    total: statsData?.postStats?.total || 0,
    stories: statsData?.postStats?.stories || 0,
    notices: statsData?.postStats?.notices || 0,
    exchanges: statsData?.postStats?.exchanges || 0,
    events: statsData?.postStats?.events || 0,
    spotlights: statsData?.postStats?.spotlights || 0,
    references: statsData?.postStats?.references || 0,
  };

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-6">
          <div className="flex items-center justify-between">
            <h1 className="text-3xl font-bold text-stone-900 mb-2">Posts</h1>
            <a
              href="/admin/posts/new"
              className="inline-flex items-center gap-1.5 px-4 py-2 text-sm font-medium text-white bg-admin-accent hover:bg-admin-accent-hover rounded-lg transition-colors"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4"><path d="M10.75 4.75a.75.75 0 0 0-1.5 0v4.5h-4.5a.75.75 0 0 0 0 1.5h4.5v4.5a.75.75 0 0 0 1.5 0v-4.5h4.5a.75.75 0 0 0 0-1.5h-4.5v-4.5Z" /></svg>
              New Post
            </a>
          </div>
          <div className="flex gap-2 mt-3">
            {([
              { key: "pending_approval", label: "Pending" },
              { key: "active", label: "Active" },
              { key: "rejected", label: "Rejected" },
            ] as const).map((s) => (
              <button
                key={s.key}
                className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                  selectedStatus === s.key
                    ? "bg-stone-900 text-white"
                    : "bg-stone-100 text-stone-700 hover:bg-stone-200"
                }`}
                onClick={() => setSelectedStatus(s.key)}
              >
                {s.label}
              </button>
            ))}
          </div>
        </div>

        {/* Search */}
        <div className="mb-4">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              setSearchQuery(searchInput);
            }}
            className="flex gap-2"
          >
            <input
              type="text"
              placeholder="Search posts by title or description..."
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              className="flex-1 px-4 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
            />
            <button
              type="submit"
              className="px-4 py-2 bg-stone-900 text-white rounded-lg text-sm font-medium hover:bg-stone-800 transition-colors"
            >
              Search
            </button>
            {searchQuery && (
              <button
                type="button"
                onClick={() => {
                  setSearchInput("");
                  setSearchQuery("");
                }}
                className="px-4 py-2 bg-stone-100 text-stone-700 rounded-lg text-sm font-medium hover:bg-stone-200 transition-colors"
              >
                Clear
              </button>
            )}
          </form>
        </div>

        {/* Zip Code Proximity Filter */}
        <div className="mb-4">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              setZipCode(zipInput);
            }}
            className="flex gap-2 items-center"
          >
            <input
              type="text"
              placeholder="Zip code (e.g., 55401)"
              value={zipInput}
              onChange={(e) => setZipInput(e.target.value.replace(/\D/g, "").slice(0, 5))}
              className="w-40 px-4 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
              maxLength={5}
            />
            <select
              value={radiusMiles}
              onChange={(e) => {
                setRadiusMiles(Number(e.target.value));
                if (zipCode) pagination.reset();
              }}
              className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
              disabled={!zipInput && !zipCode}
            >
              <option value={5}>5 miles</option>
              <option value={10}>10 miles</option>
              <option value={25}>25 miles</option>
              <option value={50}>50 miles</option>
            </select>
            <button
              type="submit"
              disabled={!zipInput || zipInput.length < 5}
              className="px-4 py-2 bg-stone-900 text-white rounded-lg text-sm font-medium hover:bg-stone-800 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Filter by Location
            </button>
            {zipCode && (
              <button
                type="button"
                onClick={() => {
                  setZipInput("");
                  setZipCode("");
                }}
                className="px-4 py-2 bg-stone-100 text-stone-700 rounded-lg text-sm font-medium hover:bg-stone-200 transition-colors"
              >
                Clear
              </button>
            )}
          </form>
        </div>

        {/* Stats Dashboard - Post Types */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
          <button
            className={`bg-white border-2 rounded-lg p-4 text-left transition-all ${
              selectedType === "all"
                ? "border-amber-500 shadow-md"
                : "border-stone-200 hover:border-stone-300"
            }`}
            onClick={() => setSelectedType("all")}
          >
            <div className="text-2xl font-bold text-stone-900">{stats.total}</div>
            <div className="text-sm text-stone-600">All Types</div>
          </button>

          {([
            { key: "story" as const, label: "Stories", count: stats.stories, color: "indigo" },
            { key: "notice" as const, label: "Notices", count: stats.notices, color: "amber" },
            { key: "exchange" as const, label: "Exchanges", count: stats.exchanges, color: "blue" },
            { key: "event" as const, label: "Events", count: stats.events, color: "green" },
            { key: "spotlight" as const, label: "Spotlights", count: stats.spotlights, color: "purple" },
            { key: "reference" as const, label: "References", count: stats.references, color: "stone" },
          ]).map((t) => (
            <button
              key={t.key}
              className={`bg-white border-2 rounded-lg p-4 text-left transition-all ${
                selectedType === t.key
                  ? `border-${t.color}-500 shadow-md`
                  : "border-stone-200 hover:border-stone-300"
              }`}
              onClick={() => setSelectedType(t.key)}
            >
              <div className={`text-2xl font-bold text-${t.color}-700`}>{t.count}</div>
              <div className="text-sm text-stone-600">{t.label}</div>
            </button>
          ))}
        </div>

        {/* Active Filters */}
        {(selectedType !== "all" || searchQuery || zipCode) && (
          <div className="mb-4 flex gap-2 flex-wrap">
            {searchQuery && (
              <span className="inline-flex items-center gap-2 px-3 py-1 bg-blue-100 text-blue-800 rounded-full text-sm">
                Search: <span className="font-semibold">{searchQuery}</span>
                <button onClick={() => { setSearchInput(""); setSearchQuery(""); }} className="hover:text-blue-900">
                  {"\u2715"}
                </button>
              </span>
            )}
            {selectedType !== "all" && (
              <span className="inline-flex items-center gap-2 px-3 py-1 bg-amber-100 text-amber-800 rounded-full text-sm">
                Type: <span className="font-semibold capitalize">{selectedType}</span>
                <button onClick={() => setSelectedType("all")} className="hover:text-amber-900">
                  {"\u2715"}
                </button>
              </span>
            )}
            {zipCode && (
              <span className="inline-flex items-center gap-2 px-3 py-1 bg-green-100 text-green-800 rounded-full text-sm">
                Near: <span className="font-semibold">{zipCode} ({radiusMiles} mi)</span>
                <button onClick={() => { setZipInput(""); setZipCode(""); }} className="hover:text-green-900">
                  {"\u2715"}
                </button>
              </span>
            )}
          </div>
        )}

        {/* Loading State */}
        {isLoading && posts.length === 0 && (
          <AdminLoader label="Loading posts..." />
        )}

        {/* Error State */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            <strong>Error:</strong> {error.message}
          </div>
        )}

        {/* Empty State */}
        {!isLoading && !error && posts.length === 0 && (
          <div className="bg-white border border-stone-200 rounded-lg p-12 text-center">
            <h3 className="text-xl font-semibold text-stone-900 mb-2">
              {selectedStatus === "pending_approval" ? "All caught up!" : "No posts found"}
            </h3>
            <p className="text-stone-600">
              No {selectedStatus === "pending_approval" ? "pending" : selectedStatus} posts
              {selectedType !== "all" && ` for ${selectedType}`}.
            </p>
          </div>
        )}

        {/* Posts Grid */}
        {!isLoading && !error && posts.length > 0 && (
          <>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">
              {posts.map((post) => (
                <div key={post.id} className="relative">
                  {/* Distance badge overlay */}
                  {post.distanceMiles != null && (
                    <div className="absolute top-2 right-2 z-10">
                      <span className="text-xs font-medium px-2 py-1 rounded bg-green-100 text-green-800">
                        {post.distanceMiles < 1
                          ? "< 1 mi"
                          : `${post.distanceMiles.toFixed(1)} mi`}
                      </span>
                    </div>
                  )}
                  <PostReviewCard
                    post={post}
                    onApprove={selectedStatus === "pending_approval" ? handleApprove : undefined}
                    onReject={selectedStatus === "pending_approval" ? handleReject : undefined}
                    isApproving={approvingId === post.id}
                    isRejecting={rejectingId === post.id}
                  />
                </div>
              ))}
            </div>

            {/* Pagination */}
            <PaginationControls
              pageInfo={pageInfo}
              totalCount={totalCount}
              currentPage={pagination.currentPage}
              pageSize={pagination.pageSize}
              onNextPage={pagination.goToNextPage}
              onPreviousPage={pagination.goToPreviousPage}
              loading={isLoading}
            />
          </>
        )}
      </div>
    </div>
  );
}
