"use client";

import { useState, useMemo } from "react";
import Link from "next/link";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { DashboardQuery } from "@/lib/graphql/dashboard";
import { BatchGenerateEditionsMutation } from "@/lib/graphql/editions";

// ─── Week helpers ────────────────────────────────────────────────────────────

function getWeekBounds(): { start: string; end: string } {
  const d = new Date();
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
  const [{ data, fetching }] = useQuery({ query: DashboardQuery });

  // Batch generate state
  const [batchResult, setBatchResult] = useState<{
    created: number;
    failed: number;
    totalCounties: number;
  } | null>(null);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [{ fetching: generating }, batchGenerate] = useMutation(
    BatchGenerateEditionsMutation
  );

  // Derive stats from latest editions
  const stats = useMemo(() => {
    const editions = data?.latestEditions ?? [];
    let draft = 0,
      inReview = 0,
      approved = 0,
      published = 0;
    for (const e of editions) {
      if (e.status === "draft") draft++;
      else if (e.status === "in_review") inReview++;
      else if (e.status === "approved") approved++;
      else if (e.status === "published") published++;
    }
    return {
      draft,
      inReview,
      approved,
      published,
      total: editions.length,
    };
  }, [data]);

  const needsReview = stats.draft + stats.inReview;
  const kanbanCount = needsReview + stats.approved;

  // Workflow guidance
  const guidance = useMemo(() => {
    if (stats.total === 0) return null;

    if (kanbanCount === 0) {
      return {
        message: "All counties are published and up to date.",
        tone: "success" as const,
      };
    }
    if (stats.approved === kanbanCount) {
      return {
        message: `All ${stats.approved} editions reviewed — ready to publish!`,
        tone: "ready" as const,
      };
    }
    if (needsReview > 0) {
      const reviewed = stats.approved;
      if (reviewed > 0) {
        return {
          message: `${reviewed} of ${kanbanCount} editions reviewed`,
          tone: "progress" as const,
        };
      }
      return {
        message: `${needsReview} edition${needsReview !== 1 ? "s" : ""} ready for review`,
        tone: "action" as const,
      };
    }
    return null;
  }, [stats, kanbanCount, needsReview]);

  const handleGenerate = async () => {
    setBatchError(null);
    setBatchResult(null);
    const bounds = getWeekBounds();
    try {
      const result = await batchGenerate(
        { periodStart: bounds.start, periodEnd: bounds.end },
        { additionalTypenames: ["Edition", "EditionConnection"] }
      );
      if (result.error) throw result.error;
      if (result.data?.batchGenerateEditions) {
        setBatchResult(result.data.batchGenerateEditions);
      }
    } catch (err: any) {
      setBatchError(err.message || "Batch generation failed");
    }
  };

  if (fetching) {
    return <AdminLoader label="Loading dashboard..." />;
  }

  return (
    <div className="min-h-screen bg-[#FDFCFA] p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold text-stone-900 mb-1">
              Dashboard
            </h1>
            <p className="text-stone-500">
              {stats.published + stats.approved} of {stats.total || 87} counties
              up to date
            </p>
          </div>
          <button
            onClick={handleGenerate}
            disabled={generating}
            className="px-4 py-2 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
          >
            {generating ? "Generating..." : "Generate This Week"}
          </button>
        </div>

        {/* Batch result / error */}
        {batchError && (
          <div className="mb-6 bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm flex items-center justify-between">
            {batchError}
            <button
              onClick={() => setBatchError(null)}
              className="ml-2 font-medium hover:underline"
            >
              Dismiss
            </button>
          </div>
        )}
        {batchResult && (
          <div className="mb-6 bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm flex items-center justify-between">
            <span>
              Created{" "}
              <span className="font-semibold">{batchResult.created}</span>{" "}
              editions
              {batchResult.failed > 0 && (
                <>
                  ,{" "}
                  <span className="font-semibold text-red-600">
                    {batchResult.failed}
                  </span>{" "}
                  failed
                </>
              )}{" "}
              out of {batchResult.totalCounties} counties.
            </span>
            <button
              onClick={() => setBatchResult(null)}
              className="ml-2 font-medium hover:underline"
            >
              Dismiss
            </button>
          </div>
        )}

        {/* Workflow guidance banner */}
        {guidance && guidance.tone !== "success" && (
          <Link
            href="/admin/workflow"
            className={`block mb-6 rounded-lg px-5 py-4 transition-colors ${
              guidance.tone === "ready"
                ? "bg-emerald-50 border border-emerald-200 hover:bg-emerald-100"
                : guidance.tone === "action"
                  ? "bg-amber-50 border border-amber-200 hover:bg-amber-100"
                  : "bg-stone-50 border border-stone-200 hover:bg-stone-100"
            }`}
          >
            <div className="flex items-center justify-between">
              <span
                className={`font-semibold text-lg ${
                  guidance.tone === "ready"
                    ? "text-emerald-800"
                    : guidance.tone === "action"
                      ? "text-amber-800"
                      : "text-stone-800"
                }`}
              >
                {guidance.message}
              </span>
              <span
                className={`text-sm font-medium ${
                  guidance.tone === "ready"
                    ? "text-emerald-600"
                    : guidance.tone === "action"
                      ? "text-amber-600"
                      : "text-stone-600"
                }`}
              >
                Go to Review Board &rarr;
              </span>
            </div>
          </Link>
        )}
        {guidance && guidance.tone === "success" && (
          <div className="mb-6 bg-green-50 border border-green-200 rounded-lg px-5 py-4">
            <span className="text-green-800 font-semibold text-lg">
              {guidance.message}
            </span>
          </div>
        )}

        {/* Stat cards */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <StatCard
            value={stats.draft}
            label="Ready for Review"
            color="bg-yellow-500"
          />
          <StatCard
            value={stats.inReview}
            label="In Review"
            color="bg-amber-500"
          />
          <StatCard
            value={stats.approved}
            label="Approved"
            color="bg-emerald-500"
            subtitle={stats.approved > 0 ? "Ready to publish" : undefined}
          />
          <StatCard
            value={stats.published}
            label="Published"
            color="bg-green-500"
            subtitle={`of ${stats.total || 87} counties`}
          />
        </div>

        {/* Quick actions */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
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
