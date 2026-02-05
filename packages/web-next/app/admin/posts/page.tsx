"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { useGraphQL, graphqlMutateClient, invalidateAllMatchingQuery } from "@/lib/graphql/client";
import { GET_PENDING_POSTS, GET_PENDING_POSTS_STATS } from "@/lib/graphql/queries";
import { APPROVE_POST, REJECT_POST } from "@/lib/graphql/mutations";
import { useCursorPagination } from "@/lib/hooks/useCursorPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { PostReviewCard } from "@/components/admin/PostReviewCard";
import type { Post, GetListingsResult, PendingPostsStatsResult } from "@/lib/types";

type PostTypeFilter = "all" | "service" | "opportunity" | "business";
type SourceFilter = "all" | "USER_SUBMITTED" | "SCRAPED";

export default function PostsPage() {
  const [selectedType, setSelectedType] = useState<PostTypeFilter>("all");
  const [selectedSource, setSelectedSource] = useState<SourceFilter>("all");
  const [approvingId, setApprovingId] = useState<string | null>(null);
  const [rejectingId, setRejectingId] = useState<string | null>(null);

  const pagination = useCursorPagination({ pageSize: 10 });

  // Reset pagination when filters change
  useEffect(() => {
    pagination.reset();
  }, [selectedType, selectedSource]);

  // Fetch stats
  const { data: statsData } = useGraphQL<PendingPostsStatsResult>(
    GET_PENDING_POSTS_STATS,
    undefined,
    { revalidateOnFocus: false }
  );

  // Fetch posts with cursor pagination and filters
  const {
    data,
    isLoading,
    error,
    mutate: refetch,
  } = useGraphQL<GetListingsResult>(
    GET_PENDING_POSTS,
    {
      postType: selectedType === "all" ? null : selectedType,
      submissionType: selectedSource === "all" ? null : selectedSource,
      ...pagination.variables,
    },
    { revalidateOnFocus: false }
  );

  const handleApprove = async (postId: string) => {
    if (!confirm("Are you sure you want to approve this post?")) return;

    setApprovingId(postId);
    try {
      await graphqlMutateClient(APPROVE_POST, { listingId: postId });
      invalidateAllMatchingQuery(GET_PENDING_POSTS);
      invalidateAllMatchingQuery(GET_PENDING_POSTS_STATS);
      refetch();
    } catch (err) {
      console.error("Failed to approve post:", err);
      alert("Failed to approve post. Check console for details.");
    } finally {
      setApprovingId(null);
    }
  };

  const handleReject = async (postId: string, reason?: string) => {
    setRejectingId(postId);
    try {
      await graphqlMutateClient(REJECT_POST, {
        listingId: postId,
        reason: reason || "Rejected by admin",
      });
      invalidateAllMatchingQuery(GET_PENDING_POSTS);
      invalidateAllMatchingQuery(GET_PENDING_POSTS_STATS);
      refetch();
    } catch (err) {
      console.error("Failed to reject post:", err);
      alert("Failed to reject post. Check console for details.");
    } finally {
      setRejectingId(null);
    }
  };

  const handleEdit = (post: Post) => {
    // Navigate to post detail page for editing
    window.location.href = `/admin/posts/${post.id}`;
  };

  const posts = data?.listings?.nodes || [];
  const totalCount = data?.listings?.totalCount || 0;
  const pageInfo = data?.listings?.pageInfo || { hasNextPage: false };
  const fullPageInfo = pagination.buildPageInfo(
    pageInfo.hasNextPage,
    pageInfo.startCursor,
    pageInfo.endCursor
  );

  const stats = {
    total: statsData?.allPending?.totalCount || 0,
    services: statsData?.pendingServices?.totalCount || 0,
    opportunities: statsData?.pendingOpportunities?.totalCount || 0,
    businesses: statsData?.pendingBusinesses?.totalCount || 0,
    userSubmitted: statsData?.pendingUserSubmitted?.totalCount || 0,
    scraped: statsData?.pendingScraped?.totalCount || 0,
  };

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-stone-900 mb-2">Post Approval Queue</h1>
          <p className="text-stone-600">
            Review and approve posts from users and the intelligent crawler
          </p>
        </div>

        {/* Stats Dashboard - Post Types */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
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

          <button
            className={`bg-white border-2 rounded-lg p-4 text-left transition-all ${
              selectedType === "service"
                ? "border-blue-500 shadow-md"
                : "border-stone-200 hover:border-stone-300"
            }`}
            onClick={() => setSelectedType("service")}
          >
            <div className="text-2xl font-bold text-blue-700">{stats.services}</div>
            <div className="text-sm text-stone-600">Services</div>
          </button>

          <button
            className={`bg-white border-2 rounded-lg p-4 text-left transition-all ${
              selectedType === "opportunity"
                ? "border-green-500 shadow-md"
                : "border-stone-200 hover:border-stone-300"
            }`}
            onClick={() => setSelectedType("opportunity")}
          >
            <div className="text-2xl font-bold text-green-700">{stats.opportunities}</div>
            <div className="text-sm text-stone-600">Opportunities</div>
          </button>

          <button
            className={`bg-white border-2 rounded-lg p-4 text-left transition-all ${
              selectedType === "business"
                ? "border-purple-500 shadow-md"
                : "border-stone-200 hover:border-stone-300"
            }`}
            onClick={() => setSelectedType("business")}
          >
            <div className="text-2xl font-bold text-purple-700">{stats.businesses}</div>
            <div className="text-sm text-stone-600">Businesses</div>
          </button>
        </div>

        {/* Source Filter */}
        <div className="flex gap-2 mb-6">
          <button
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              selectedSource === "all"
                ? "bg-stone-900 text-white"
                : "bg-white border border-stone-200 text-stone-700 hover:border-stone-300"
            }`}
            onClick={() => setSelectedSource("all")}
          >
            All Sources ({stats.total})
          </button>
          <button
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              selectedSource === "USER_SUBMITTED"
                ? "bg-stone-900 text-white"
                : "bg-white border border-stone-200 text-stone-700 hover:border-stone-300"
            }`}
            onClick={() => setSelectedSource("USER_SUBMITTED")}
          >
            User Submitted ({stats.userSubmitted})
          </button>
          <button
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-all ${
              selectedSource === "SCRAPED"
                ? "bg-stone-900 text-white"
                : "bg-white border border-stone-200 text-stone-700 hover:border-stone-300"
            }`}
            onClick={() => setSelectedSource("SCRAPED")}
          >
            Scraped ({stats.scraped})
          </button>
        </div>

        {/* Active Filters */}
        {(selectedType !== "all" || selectedSource !== "all") && (
          <div className="mb-4 flex gap-2 flex-wrap">
            {selectedType !== "all" && (
              <span className="inline-flex items-center gap-2 px-3 py-1 bg-amber-100 text-amber-800 rounded-full text-sm">
                Type: <span className="font-semibold capitalize">{selectedType}</span>
                <button onClick={() => setSelectedType("all")} className="hover:text-amber-900">
                  {"\u2715"}
                </button>
              </span>
            )}
            {selectedSource !== "all" && (
              <span className="inline-flex items-center gap-2 px-3 py-1 bg-stone-200 text-stone-800 rounded-full text-sm">
                Source: <span className="font-semibold">{selectedSource === "USER_SUBMITTED" ? "User" : "Scraped"}</span>
                <button onClick={() => setSelectedSource("all")} className="hover:text-stone-900">
                  {"\u2715"}
                </button>
              </span>
            )}
          </div>
        )}

        {/* Loading State */}
        {isLoading && posts.length === 0 && (
          <div className="text-center py-12">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-amber-600"></div>
            <p className="mt-2 text-stone-600">Loading posts...</p>
          </div>
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
            <div className="text-6xl mb-4">{"\u{1F389}"}</div>
            <h3 className="text-xl font-semibold text-stone-900 mb-2">All caught up!</h3>
            <p className="text-stone-600">
              No pending posts to review
              {selectedType !== "all" && ` for ${selectedType}`}
              {selectedSource !== "all" && ` from ${selectedSource === "USER_SUBMITTED" ? "users" : "scraper"}`}.
            </p>
          </div>
        )}

        {/* Posts Grid */}
        {!isLoading && !error && posts.length > 0 && (
          <>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">
              {posts.map((post) => (
                <div key={post.id} className="relative">
                  {/* Source badge overlay */}
                  <div className="absolute top-2 right-2 z-10">
                    <span className={`text-xs font-medium px-2 py-1 rounded ${
                      post.submissionType === "USER_SUBMITTED"
                        ? "bg-amber-100 text-amber-800"
                        : "bg-stone-100 text-stone-700"
                    }`}>
                      {post.submissionType === "USER_SUBMITTED" ? "User" : "Scraped"}
                    </span>
                  </div>
                  <PostReviewCard
                    post={post}
                    onApprove={handleApprove}
                    onReject={handleReject}
                    onEdit={handleEdit}
                    isApproving={approvingId === post.id}
                    isRejecting={rejectingId === post.id}
                  />
                </div>
              ))}
            </div>

            {/* Pagination */}
            <PaginationControls
              pageInfo={fullPageInfo}
              totalCount={totalCount}
              currentPage={pagination.currentPage}
              pageSize={pagination.pageSize}
              onNextPage={() => pagination.goToNextPage(pageInfo.endCursor ?? null)}
              onPreviousPage={pagination.goToPreviousPage}
              loading={isLoading}
            />
          </>
        )}

        {/* Helpful Tips */}
        <div className="mt-6 bg-white border border-amber-200 rounded-lg p-6">
          <h3 className="font-semibold text-amber-900 mb-3">Review Tips</h3>
          <ul className="text-sm text-stone-700 space-y-2 list-disc list-inside">
            <li><strong>Services</strong> offer help (legal aid, healthcare, food pantries)</li>
            <li><strong>Opportunities</strong> need help (volunteers, donations, partnerships)</li>
            <li><strong>Businesses</strong> donate proceeds to causes</li>
            <li>Click <strong>Show more</strong> to see full details and type-specific fields</li>
            <li>Use <strong>Edit</strong> to view and improve text before approving</li>
            <li>Check source URL to verify accuracy</li>
            <li>Reject spam, duplicates, or irrelevant content</li>
          </ul>
        </div>
      </div>
    </div>
  );
}
