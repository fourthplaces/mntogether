"use client";

import { useState, useMemo } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  EditionsListQuery,
  CountiesQuery,
  BatchGenerateEditionsMutation,
} from "@/lib/graphql/editions";
import {
  formatPeriodLabel,
  getWeeksOld,
  getStalenessLevel,
  STALENESS_TEXT,
} from "@/lib/staleness";

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

// ─── Status config ───────────────────────────────────────────────────────────

const STATUS_FILTERS = [
  { value: "", label: "All" },
  { value: "draft", label: "Ready for Review" },
  { value: "in_review", label: "In Review" },
  { value: "approved", label: "Approved" },
  { value: "published", label: "Published" },
  { value: "archived", label: "Archived" },
];

const STATUS_BADGE_STYLES: Record<string, string> = {
  draft: "bg-yellow-100 text-yellow-800",
  in_review: "bg-amber-100 text-amber-800",
  approved: "bg-emerald-100 text-emerald-800",
  published: "bg-green-100 text-green-800",
  archived: "bg-stone-100 text-stone-600",
};

const STATUS_LABELS: Record<string, string> = {
  draft: "Ready for Review",
  in_review: "In Review",
  approved: "Approved",
  published: "Published",
  archived: "Archived",
};

// ─── Component ───────────────────────────────────────────────────────────────

export default function EditionsPage() {
  const router = useRouter();
  const [countyFilter, setCountyFilter] = useState<string>("");
  const [statusFilter, setStatusFilter] = useState<string>("");

  // ─── Queries ────────────────────────────────────────────────────────
  const [{ data: countiesData }] = useQuery({ query: CountiesQuery });
  const [{ data, fetching, error }] = useQuery({
    query: EditionsListQuery,
    variables: {
      countyId: countyFilter || null,
      status: statusFilter || null,
      limit: 50,
      offset: 0,
    },
  });

  const counties = useMemo(() => {
    const list = countiesData?.counties || [];
    return [...list].sort((a, b) => a.name.localeCompare(b.name));
  }, [countiesData]);

  const editions = data?.editions?.editions || [];
  const totalCount = data?.editions?.totalCount ?? 0;

  // ─── Batch generate (one-click, auto week bounds) ──────────────────
  const [batchResult, setBatchResult] = useState<{
    created: number;
    failed: number;
    totalCounties: number;
  } | null>(null);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [{ fetching: batching }, batchGenerate] = useMutation(
    BatchGenerateEditionsMutation
  );

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

  // ─── Status badge ───────────────────────────────────────────────────
  const statusBadge = (status: string) => (
    <span
      className={`px-2 py-0.5 text-xs rounded-full font-medium ${
        STATUS_BADGE_STYLES[status] || "bg-stone-100 text-stone-600"
      }`}
    >
      {STATUS_LABELS[status] || status}
    </span>
  );

  // ─── Render ─────────────────────────────────────────────────────────
  if (fetching && editions.length === 0 && !data) {
    return <AdminLoader label="Loading editions..." />;
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-3xl font-bold text-stone-900">Editions</h1>
            <p className="text-stone-500 text-sm mt-1">
              {totalCount} edition{totalCount !== 1 ? "s" : ""} found
            </p>
          </div>
          <button
            onClick={handleGenerate}
            disabled={batching}
            className="px-4 py-2 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
          >
            {batching ? "Generating..." : "Generate This Week"}
          </button>
        </div>

        {/* Batch result / error */}
        {batchError && (
          <div className="mb-4 bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm flex items-center justify-between">
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
          <div className="mb-4 bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm flex items-center justify-between">
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

        {/* Filters */}
        <div className="flex gap-3 mb-6">
          <select
            value={countyFilter}
            onChange={(e) => setCountyFilter(e.target.value)}
            className="px-3 py-2 border border-stone-300 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-amber-500"
          >
            <option value="">All counties</option>
            {counties.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          <div className="flex rounded-lg border border-stone-300 overflow-hidden">
            {STATUS_FILTERS.map((s) => (
              <button
                key={s.value}
                onClick={() => setStatusFilter(s.value)}
                className={`px-3 py-2 text-sm font-medium transition-colors ${
                  statusFilter === s.value
                    ? "bg-amber-100 text-amber-800"
                    : "bg-white text-stone-600 hover:bg-stone-50"
                }`}
              >
                {s.label}
              </button>
            ))}
          </div>
        </div>

        {/* Error */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            Error: {error.message}
          </div>
        )}

        {/* Table */}
        {editions.length === 0 ? (
          <div className="text-stone-500 text-center py-12">
            No editions found. Use &ldquo;Generate This Week&rdquo; to create
            broadsheets.
          </div>
        ) : (
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <table className="min-w-full divide-y divide-stone-200">
              <thead className="bg-stone-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    County
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Period
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Rows
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Created
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-stone-200">
                {editions.map((ed) => {
                  const weeksOld = getWeeksOld(ed.periodEnd);
                  const level = getStalenessLevel(weeksOld);
                  const periodTextClass = STALENESS_TEXT[level];

                  return (
                    <tr
                      key={ed.id}
                      onClick={() => router.push(`/admin/editions/${ed.id}`)}
                      className="hover:bg-stone-50 cursor-pointer"
                    >
                      <td className="px-6 py-4 whitespace-nowrap">
                        <div className="font-medium text-stone-900">
                          {ed.county.name}
                        </div>
                        {ed.title && (
                          <div className="text-xs text-stone-500 truncate max-w-xs">
                            {ed.title}
                          </div>
                        )}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        <span
                          className={`text-sm font-medium ${periodTextClass}`}
                        >
                          {level === "alert" && (
                            <svg
                              className="w-3.5 h-3.5 inline-block mr-1 -mt-px"
                              fill="none"
                              stroke="currentColor"
                              viewBox="0 0 24 24"
                            >
                              <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                strokeWidth={2}
                                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.5c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z"
                              />
                            </svg>
                          )}
                          {formatPeriodLabel(ed.periodEnd)}
                        </span>
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap">
                        {statusBadge(ed.status)}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-500">
                        {ed.rows.length} row{ed.rows.length !== 1 ? "s" : ""}
                      </td>
                      <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-500">
                        {new Date(ed.createdAt).toLocaleDateString()}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
