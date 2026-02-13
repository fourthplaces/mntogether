"use client";

import { useState, useEffect, useRef } from "react";
import { useRestate, callObject, callService, invalidateService, invalidateObject } from "@/lib/restate/client";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { PostReviewCard } from "@/components/admin/PostReviewCard";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { PostList, PostStats } from "@/lib/restate/types";

type ScoreFilter = "all" | "high" | "review" | "noise" | "unscored";

type PostTypeFilter = "all" | "service" | "opportunity" | "business";
type SourceFilter = "all" | "USER_SUBMITTED" | "SCRAPED";
type StatusFilter = "pending_approval" | "active" | "rejected";

export default function PostsPage() {
  const [selectedStatus, setSelectedStatus] = useState<StatusFilter>("pending_approval");
  const [selectedType, setSelectedType] = useState<PostTypeFilter>("all");
  const [selectedSource, setSelectedSource] = useState<SourceFilter>("all");
  const [searchInput, setSearchInput] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [zipInput, setZipInput] = useState("");
  const [zipCode, setZipCode] = useState("");
  const [radiusMiles, setRadiusMiles] = useState<number>(25);
  const [approvingId, setApprovingId] = useState<string | null>(null);
  const [rejectingId, setRejectingId] = useState<string | null>(null);
  const [scoreFilter, setScoreFilter] = useState<ScoreFilter>("all");
  const [moreMenuOpen, setMoreMenuOpen] = useState(false);
  const [scoring, setScoring] = useState(false);
  const [scoreResult, setScoreResult] = useState<string | null>(null);
  const moreMenuRef = useRef<HTMLDivElement>(null);

  const pagination = useOffsetPagination({ pageSize: 10 });

  // Reset pagination when filters change
  useEffect(() => {
    pagination.reset();
  }, [selectedStatus, selectedType, selectedSource, searchQuery, zipCode, radiusMiles]);

  // Close more menu on click outside
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (moreMenuRef.current && !moreMenuRef.current.contains(e.target as Node)) {
        setMoreMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  // Fetch stats
  const { data: statsData } = useRestate<PostStats>(
    "Posts", "stats", { status: selectedStatus },
    { revalidateOnFocus: false }
  );

  // Fetch posts with offset pagination and filters
  const {
    data,
    isLoading,
    error,
    mutate: refetch,
  } = useRestate<PostList>(
    "Posts", "list",
    {
      status: selectedStatus,
      post_type: selectedType === "all" ? null : selectedType,
      submission_type: selectedSource === "all" ? null : selectedSource,
      search: searchQuery || null,
      zip_code: zipCode || null,
      radius_miles: zipCode ? radiusMiles : null,
      ...pagination.variables,
    },
    { revalidateOnFocus: false }
  );

  const handleApprove = async (postId: string) => {
    setApprovingId(postId);
    try {
      await callObject("Post", postId, "approve", {});
      invalidateService("Posts");
      invalidateObject("Post", postId);
      refetch();
    } catch (err) {
      console.error("Failed to approve post:", err);
    } finally {
      setApprovingId(null);
    }
  };

  const handleReject = async (postId: string, reason?: string) => {
    setRejectingId(postId);
    try {
      await callObject("Post", postId, "reject", {
        reason: reason || "Rejected by admin",
      });
      invalidateService("Posts");
      invalidateObject("Post", postId);
      refetch();
    } catch (err) {
      console.error("Failed to reject post:", err);
    } finally {
      setRejectingId(null);
    }
  };

  const handleScoreAll = async () => {
    setMoreMenuOpen(false);
    setScoring(true);
    setScoreResult(null);
    try {
      const result = await callService<{ scored: number; failed: number; remaining: number }>(
        "Posts", "batch_score_posts", { limit: 200 }
      );
      setScoreResult(`Scored ${result.scored} posts${result.failed > 0 ? `, ${result.failed} failed` : ""}${result.remaining > 0 ? `, ${result.remaining} remaining` : ""}`);
      refetch();
    } catch (err: any) {
      setScoreResult(`Error: ${err.message || "Failed to score posts"}`);
    } finally {
      setScoring(false);
    }
  };

  const matchesScoreFilter = (post: { relevance_score?: number | null }) => {
    if (scoreFilter === "all") return true;
    if (scoreFilter === "unscored") return post.relevance_score == null;
    if (post.relevance_score == null) return false;
    if (scoreFilter === "high") return post.relevance_score >= 8;
    if (scoreFilter === "review") return post.relevance_score >= 5 && post.relevance_score <= 7;
    if (scoreFilter === "noise") return post.relevance_score <= 4;
    return true;
  };

  const allPosts = data?.posts || [];
  const posts = allPosts.filter(matchesScoreFilter);
  const totalCount = data?.total_count || 0;
  const hasNextPage = data?.has_next_page || false;
  const pageInfo = pagination.buildPageInfo(hasNextPage);

  const stats = {
    total: statsData?.total || 0,
    services: statsData?.services || 0,
    opportunities: statsData?.opportunities || 0,
    businesses: statsData?.businesses || 0,
    userSubmitted: statsData?.user_submitted || 0,
    scraped: statsData?.scraped || 0,
  };

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Score result banner */}
        {scoreResult && (
          <div className={`mb-4 px-4 py-3 rounded-lg text-sm ${scoreResult.startsWith("Error") ? "bg-red-50 text-red-700 border border-red-200" : "bg-green-50 text-green-700 border border-green-200"}`}>
            {scoreResult}
            <button onClick={() => setScoreResult(null)} className="ml-2 font-medium hover:underline">Dismiss</button>
          </div>
        )}

        {/* Header */}
        <div className="mb-6">
          <div className="flex items-center justify-between">
            <h1 className="text-3xl font-bold text-stone-900 mb-2">Posts</h1>
            <div className="relative" ref={moreMenuRef}>
              <button
                onClick={() => setMoreMenuOpen(!moreMenuOpen)}
                disabled={scoring}
                className="p-2 rounded-lg text-stone-500 hover:bg-stone-100 hover:text-stone-700 transition-colors disabled:opacity-50"
                title="More actions"
              >
                {scoring ? (
                  <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" /><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" /></svg>
                ) : (
                  <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20"><circle cx="4" cy="10" r="2" /><circle cx="10" cy="10" r="2" /><circle cx="16" cy="10" r="2" /></svg>
                )}
              </button>
              {moreMenuOpen && (
                <div className="absolute right-0 mt-1 w-56 bg-white rounded-lg shadow-lg border border-stone-200 py-1 z-20">
                  <button
                    onClick={handleScoreAll}
                    className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50"
                  >
                    Score All Unscored Posts
                  </button>
                </div>
              )}
            </div>
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

        {/* Source Filter + Score Filter */}
        <div className="flex gap-2 mb-6 flex-wrap items-center">
          <button
            className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
              selectedSource === "all"
                ? "bg-amber-600 text-white"
                : "bg-stone-100 text-stone-700 hover:bg-stone-200"
            }`}
            onClick={() => setSelectedSource("all")}
          >
            All Sources ({stats.total})
          </button>
          <button
            className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
              selectedSource === "USER_SUBMITTED"
                ? "bg-amber-600 text-white"
                : "bg-stone-100 text-stone-700 hover:bg-stone-200"
            }`}
            onClick={() => setSelectedSource("USER_SUBMITTED")}
          >
            User Submitted ({stats.userSubmitted})
          </button>
          <button
            className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
              selectedSource === "SCRAPED"
                ? "bg-amber-600 text-white"
                : "bg-stone-100 text-stone-700 hover:bg-stone-200"
            }`}
            onClick={() => setSelectedSource("SCRAPED")}
          >
            Scraped ({stats.scraped})
          </button>

          <div className="w-px h-6 bg-stone-300 mx-1" />

          <select
            value={scoreFilter}
            onChange={(e) => setScoreFilter(e.target.value as ScoreFilter)}
            className="px-3 py-1.5 rounded-lg text-sm font-medium border border-stone-300 bg-white text-stone-700 focus:outline-none focus:ring-2 focus:ring-amber-500"
          >
            <option value="all">All Scores</option>
            <option value="high">High (8-10)</option>
            <option value="review">Review (5-7)</option>
            <option value="noise">Noise (1-4)</option>
            <option value="unscored">Unscored</option>
          </select>
        </div>

        {/* Active Filters */}
        {(selectedType !== "all" || selectedSource !== "all" || searchQuery || zipCode || scoreFilter !== "all") && (
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
            {selectedSource !== "all" && (
              <span className="inline-flex items-center gap-2 px-3 py-1 bg-stone-200 text-stone-800 rounded-full text-sm">
                Source: <span className="font-semibold">{selectedSource === "USER_SUBMITTED" ? "User" : "Scraped"}</span>
                <button onClick={() => setSelectedSource("all")} className="hover:text-stone-900">
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
            {scoreFilter !== "all" && (
              <span className="inline-flex items-center gap-2 px-3 py-1 bg-indigo-100 text-indigo-800 rounded-full text-sm">
                Score: <span className="font-semibold">
                  {scoreFilter === "high" ? "High (8-10)" : scoreFilter === "review" ? "Review (5-7)" : scoreFilter === "noise" ? "Noise (1-4)" : "Unscored"}
                </span>
                <button onClick={() => setScoreFilter("all")} className="hover:text-indigo-900">
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
            <div className="text-6xl mb-4">{selectedStatus === "pending_approval" ? "\u{1F389}" : "\u{1F4ED}"}</div>
            <h3 className="text-xl font-semibold text-stone-900 mb-2">
              {selectedStatus === "pending_approval" ? "All caught up!" : "No posts found"}
            </h3>
            <p className="text-stone-600">
              No {selectedStatus === "pending_approval" ? "pending" : selectedStatus} posts
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
                  <div className="absolute top-2 right-2 z-10 flex gap-1">
                    {post.distance_miles != null && (
                      <span className="text-xs font-medium px-2 py-1 rounded bg-green-100 text-green-800">
                        {post.distance_miles < 1
                          ? "< 1 mi"
                          : `${post.distance_miles.toFixed(1)} mi`}
                      </span>
                    )}
                    <span className={`text-xs font-medium px-2 py-1 rounded ${
                      post.submission_type === "USER_SUBMITTED"
                        ? "bg-amber-100 text-amber-800"
                        : "bg-stone-100 text-stone-700"
                    }`}>
                      {post.submission_type === "USER_SUBMITTED" ? "User" : "Scraped"}
                    </span>
                  </div>
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
