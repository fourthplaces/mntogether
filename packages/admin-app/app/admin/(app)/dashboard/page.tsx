"use client";

import { useMemo } from "react";
import Link from "next/link";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { DashboardQuery } from "@/lib/graphql/dashboard";
import { BatchGenerateEditionsMutation } from "@/lib/graphql/editions";
import { getWeeksOld } from "@/lib/staleness";

// ─── Week helpers (still needed for batch generate) ─────────────────────────

function getWeekBounds(date: Date): { start: string; end: string } {
  const d = new Date(date);
  const day = d.getDay();
  const diffToMonday = day === 0 ? -6 : 1 - day;
  const monday = new Date(d);
  monday.setDate(d.getDate() + diffToMonday);
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  return {
    start: monday.toISOString().split("T")[0],
    end: sunday.toISOString().split("T")[0],
  };
}

// ─── Component ───────────────────────────────────────────────────────────────

export default function DashboardPage() {
  const [{ data, fetching }] = useQuery({
    query: DashboardQuery,
  });

  const [{ fetching: generating }, batchGenerate] = useMutation(
    BatchGenerateEditionsMutation
  );

  // Derive stats from latest editions
  const { draft, inReview, approved, published, staleCount, totalEditions } = useMemo(() => {
    const editions = data?.latestEditions ?? [];
    let draft = 0, inReview = 0, approved = 0, published = 0, staleCount = 0;
    for (const e of editions) {
      if (e.status === "draft") draft++;
      else if (e.status === "in_review") inReview++;
      else if (e.status === "approved") approved++;
      else if (e.status === "published") published++;

      if (getWeeksOld(e.periodEnd) >= 2) staleCount++;
    }
    return { draft, inReview, approved, published, staleCount, totalEditions: editions.length };
  }, [data]);

  const pendingPosts = data?.pendingPosts;
  const totalPosts = data?.allPosts?.totalCount ?? 0;
  const needsReview = draft + inReview;
  const upToDate = approved + published;

  const handleBatchGenerate = async () => {
    const bounds = getWeekBounds(new Date());
    await batchGenerate(
      { periodStart: bounds.start, periodEnd: bounds.end },
      { additionalTypenames: ["Edition", "EditionConnection"] }
    );
  };

  if (fetching) {
    return <AdminLoader label="Loading dashboard..." />;
  }

  return (
    <div className="min-h-screen bg-[#FDFCFA] p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-stone-900 mb-1">
            Edition Cockpit
          </h1>
          <p className="text-stone-500">
            {upToDate} of {totalEditions || 87} counties up to date
          </p>
        </div>

        {/* Alert banner */}
        {needsReview > 0 && (
          <Link
            href="/admin/workflow"
            className="block mb-6 bg-amber-50 border border-amber-200 rounded-lg px-5 py-4 hover:bg-amber-100 transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <span className="text-amber-800 font-semibold text-lg">
                  {needsReview} edition{needsReview !== 1 ? "s" : ""} need
                  review
                </span>
                <p className="text-amber-700 text-sm mt-0.5">
                  {draft} draft, {inReview} in review
                </p>
              </div>
              <span className="text-amber-600 text-sm font-medium">
                Go to Review Board &rarr;
              </span>
            </div>
          </Link>
        )}

        {/* Stale editions warning */}
        {staleCount > 0 && (
          <Link
            href="/admin/workflow"
            className="block mb-6 bg-red-50 border border-red-200 rounded-lg px-5 py-4 hover:bg-red-100 transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <span className="text-red-800 font-semibold text-lg">
                  {staleCount} county edition{staleCount !== 1 ? "s" : ""} stale
                </span>
                <p className="text-red-700 text-sm mt-0.5">
                  2+ weeks old &mdash; consider regenerating
                </p>
              </div>
              <span className="text-red-600 text-sm font-medium">
                View on Review Board &rarr;
              </span>
            </div>
          </Link>
        )}

        {/* Edition stats cards */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <StatCard
            value={published}
            label="Live"
            color="bg-green-500"
            subtitle={`of ${totalEditions || 87} counties`}
          />
          <StatCard
            value={draft}
            label="Ready for Review"
            color="bg-yellow-500"
          />
          <StatCard
            value={inReview}
            label="In Review"
            color="bg-amber-500"
          />
          <StatCard
            value={approved}
            label="Approved"
            color="bg-emerald-500"
            subtitle={approved > 0 ? "Ready to publish" : undefined}
          />
        </div>

        {/* Quick actions */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-8">
          <Link
            href="/admin/workflow"
            className="bg-amber-600 hover:bg-amber-700 text-white rounded-lg shadow-sm p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">Review Board</div>
            <p className="text-amber-100 text-sm">
              Drag editions through the approval pipeline
            </p>
          </Link>
          <Link
            href="/admin/editions"
            className="bg-stone-700 hover:bg-stone-800 text-white rounded-lg shadow-sm p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">All Editions</div>
            <p className="text-stone-300 text-sm">
              Browse and filter all county editions
            </p>
          </Link>
          <button
            onClick={handleBatchGenerate}
            disabled={generating}
            className="bg-white hover:bg-stone-50 text-stone-800 border border-stone-200 rounded-lg shadow-sm p-5 transition-colors text-left disabled:opacity-50"
          >
            <div className="text-lg font-semibold mb-1">
              {generating ? "Generating..." : "Batch Generate"}
            </div>
            <p className="text-stone-500 text-sm">
              Auto-generate editions for all 87 counties this week
            </p>
          </button>
        </div>

        {/* Two-column bottom: pending posts + content summary */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Pending posts */}
          <div className="bg-white rounded-lg shadow-sm border border-stone-200 p-5">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-lg font-semibold text-stone-900">
                Pending Posts
              </h2>
              {(pendingPosts?.totalCount ?? 0) > 0 && (
                <Link
                  href="/admin/posts?status=pending_approval"
                  className="text-sm text-amber-600 hover:text-amber-700 font-medium"
                >
                  View all ({pendingPosts?.totalCount}) &rarr;
                </Link>
              )}
            </div>
            {pendingPosts?.posts && pendingPosts.posts.length > 0 ? (
              <div className="space-y-2">
                {pendingPosts.posts.map((post) => (
                  <Link
                    key={post.id}
                    href={`/admin/posts/${post.id}`}
                    className="block px-3 py-2 rounded-lg hover:bg-stone-50 transition-colors"
                  >
                    <div className="text-sm font-medium text-stone-900 truncate">
                      {post.title}
                    </div>
                    <div className="text-xs text-stone-400 mt-0.5">
                      {new Date(post.createdAt).toLocaleDateString()}
                    </div>
                  </Link>
                ))}
              </div>
            ) : (
              <p className="text-sm text-stone-400">
                No posts pending approval
              </p>
            )}
          </div>

          {/* Content summary */}
          <div className="bg-white rounded-lg shadow-sm border border-stone-200 p-5">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">
              Content Summary
            </h2>
            <div className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-stone-600">Total posts</span>
                <span className="text-sm font-semibold text-stone-900">
                  {totalPosts}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-stone-600">
                  Counties tracked
                </span>
                <span className="text-sm font-semibold text-stone-900">
                  {totalEditions || 87}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-stone-600">Published</span>
                <span className="text-sm font-semibold text-green-700">
                  {published}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-stone-600">
                  Pending approval
                </span>
                <span className="text-sm font-semibold text-amber-700">
                  {pendingPosts?.totalCount ?? 0}
                </span>
              </div>
              {staleCount > 0 && (
                <div className="flex items-center justify-between">
                  <span className="text-sm text-stone-600">
                    Stale editions
                  </span>
                  <span className="text-sm font-semibold text-red-600">
                    {staleCount}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// ─── StatCard ────────────────────────────────────────────────────────────────

function StatCard({
  value,
  label,
  color,
  subtitle,
}: {
  value: number;
  label: string;
  color: string;
  subtitle?: string;
}) {
  return (
    <div className="bg-white rounded-lg shadow-sm border border-stone-200 p-5">
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium text-stone-500 uppercase tracking-wide">
          {label}
        </span>
        <div className={`w-2.5 h-2.5 rounded-full ${color}`} />
      </div>
      <div className="text-3xl font-bold text-stone-900">{value}</div>
      {subtitle && (
        <div className="text-xs text-stone-400 mt-1">{subtitle}</div>
      )}
    </div>
  );
}
