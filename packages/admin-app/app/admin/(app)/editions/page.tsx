"use client";

import { useState, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  CountyDashboardQuery,
  EditionHistoryQuery,
  BatchGenerateEditionsMutation,
} from "@/lib/graphql/editions";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";

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

function formatPeriod(start: string, end: string): string {
  const s = new Date(start + "T12:00:00");
  const e = new Date(end + "T12:00:00");
  const sMonth = s.toLocaleDateString("en-US", { month: "short" });
  const eMonth = e.toLocaleDateString("en-US", { month: "short" });
  if (sMonth === eMonth) {
    return `${sMonth} ${s.getDate()}\u2013${e.getDate()}`;
  }
  return `${sMonth} ${s.getDate()} \u2013 ${eMonth} ${e.getDate()}`;
}

function liveEditionAge(periodStart: string | undefined, isStale: boolean): string {
  if (!periodStart) return "Never";
  const now = new Date();
  const day = now.getDay();
  const diffToMonday = day === 0 ? -6 : 1 - day;
  const monday = new Date(now);
  monday.setDate(now.getDate() + diffToMonday);
  monday.setHours(0, 0, 0, 0);

  const edStart = new Date(periodStart + "T12:00:00");
  const diffMs = monday.getTime() - edStart.getTime();
  const weeks = Math.round(diffMs / (7 * 24 * 60 * 60 * 1000));

  if (weeks <= 0) return isStale ? "Not yet published" : "This week";
  if (weeks === 1) return "1 week ago";
  return `${weeks} weeks ago`;
}

// ─── Status config ───────────────────────────────────────────────────────────

const STATUS_BADGE_STYLES: Record<string, string> = {
  draft: "bg-yellow-100 text-yellow-800",
  in_review: "bg-amber-100 text-amber-800",
  approved: "bg-emerald-100 text-emerald-800",
  published: "bg-green-100 text-green-800",
  archived: "bg-stone-100 text-stone-600",
};

const STATUS_LABELS: Record<string, string> = {
  draft: "Draft",
  in_review: "Reviewing",
  approved: "Approved",
  published: "Published",
  archived: "Archived",
};

const STATUS_FILTERS = [
  { value: "", label: "All" },
  { value: "draft", label: "Draft" },
  { value: "in_review", label: "Reviewing" },
  { value: "approved", label: "Approved" },
];

// ─── Freshness indicators ────────────────────────────────────────────────────

function FreshnessIndicator({ isStale, status }: { isStale: boolean; status?: string }) {
  if (!status) {
    // No edition at all
    return <span className="text-stone-400" title="No edition">—</span>;
  }
  if (status === "published" && !isStale) {
    return <span className="text-green-600" title="Published (current)">✓</span>;
  }
  if (isStale) {
    return <span className="text-amber-600" title="Stale">⚠</span>;
  }
  if (status === "approved") {
    return <span className="text-emerald-600" title="Approved">●</span>;
  }
  if (status === "in_review") {
    return <span className="text-amber-500" title="Reviewing">●</span>;
  }
  // draft
  return <span className="text-stone-400" title="Draft">○</span>;
}

// ─── Component ───────────────────────────────────────────────────────────────

export default function CountiesDashboardPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [statusFilter, setStatusFilter] = useState(searchParams.get("status") || "");
  const [searchQuery, setSearchQuery] = useState("");
  const [onlyStale, setOnlyStale] = useState(false);
  const [showGenerateModal, setShowGenerateModal] = useState(false);
  const [historyCountyId, setHistoryCountyId] = useState<string | null>(null);
  const [historyCountyName, setHistoryCountyName] = useState("");

  // ─── Queries ──────────────────────────────────────────────────────
  const [{ data, fetching, error }] = useQuery({ query: CountyDashboardQuery });
  const rows = data?.countyDashboard || [];

  // History modal query
  const [{ data: historyData, fetching: historyFetching }] = useQuery({
    query: EditionHistoryQuery,
    variables: { countyId: historyCountyId, limit: 20 },
    pause: !historyCountyId,
  });

  // Filter rows
  const filteredRows = useMemo(() => {
    let result = rows;

    // Search
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter((r) => r.county.name.toLowerCase().includes(q));
    }

    // Status filter (editorial state)
    if (statusFilter) {
      result = result.filter((r) => r.currentEdition?.status === statusFilter);
    }

    // Independent stale toggle (county-level freshness)
    if (onlyStale) {
      result = result.filter((r) => r.isStale);
    }

    return result;
  }, [rows, searchQuery, statusFilter, onlyStale]);

  // ─── Batch generate ───────────────────────────────────────────────
  const [batchResult, setBatchResult] = useState<{
    created: number;
    regenerated: number;
    skipped: number;
    failed: number;
    totalCounties: number;
  } | null>(null);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [{ fetching: batching }, batchGenerate] = useMutation(
    BatchGenerateEditionsMutation
  );

  // Preview what batch generate will do (computed from loaded dashboard data)
  const generatePreview = useMemo(() => {
    const bounds = getWeekBounds();
    let toCreate = 0;
    let toRegenerate = 0;
    let toSkip = 0;
    const skipReasons = { reviewing: 0, approved: 0, published: 0 };

    for (const row of rows) {
      const ed = row.currentEdition;
      if (!ed || ed.periodStart < bounds.start) {
        toCreate++;
      } else {
        switch (ed.status) {
          case "draft":
            toRegenerate++;
            break;
          case "in_review":
            toSkip++;
            skipReasons.reviewing++;
            break;
          case "approved":
            toSkip++;
            skipReasons.approved++;
            break;
          case "published":
            toSkip++;
            skipReasons.published++;
            break;
          default:
            toSkip++;
            break;
        }
      }
    }

    return { toCreate, toRegenerate, toSkip, skipReasons };
  }, [rows]);

  const handleGenerate = async () => {
    setBatchError(null);
    setBatchResult(null);
    const bounds = getWeekBounds();
    try {
      const result = await batchGenerate(
        { periodStart: bounds.start, periodEnd: bounds.end },
        { additionalTypenames: ["Edition", "EditionConnection", "CountyDashboardRow"] }
      );
      if (result.error) throw result.error;
      if (result.data?.batchGenerateEditions) {
        setBatchResult(result.data.batchGenerateEditions);
      }
    } catch (err: unknown) {
      setBatchError(err instanceof Error ? err.message : "Batch generation failed");
    }
  };

  // ─── Summary stats ────────────────────────────────────────────────
  const staleCount = rows.filter((r) => r.isStale).length;
  const publishedCount = rows.filter((r) => r.currentEdition?.status === "published" && !r.isStale).length;
  const inProgressCount = rows.filter((r) => {
    const s = r.currentEdition?.status;
    return s === "draft" || s === "in_review" || s === "approved";
  }).length;

  // ─── Render ───────────────────────────────────────────────────────
  if (fetching && rows.length === 0) {
    return <AdminLoader label="Loading counties..." />;
  }

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-foreground">Counties</h1>
            <p className="text-muted-foreground text-sm mt-0.5">
              {publishedCount} published · {inProgressCount} in progress · {staleCount} stale
            </p>
          </div>
          <button
            onClick={() => setShowGenerateModal(true)}
            disabled={batching}
            className="px-4 py-2 rounded-lg text-sm font-medium bg-admin-accent text-white hover:bg-admin-accent-hover disabled:opacity-50 transition-colors"
          >
            {batching ? "Generating..." : "Generate Drafts"}
          </button>
        </div>

        {/* Batch result / error */}
        {batchError && (
          <div className="mb-4 bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm flex items-center justify-between">
            {batchError}
            <button onClick={() => setBatchError(null)} className="ml-2 font-medium hover:underline">
              Dismiss
            </button>
          </div>
        )}
        {batchResult && (
          <div className="mb-4 bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm flex items-center justify-between">
            <span>
              Created <span className="font-semibold">{batchResult.created}</span>
              {batchResult.regenerated > 0 && (
                <>, regenerated <span className="font-semibold">{batchResult.regenerated}</span></>
              )}
              {batchResult.skipped > 0 && (
                <>, skipped <span className="font-semibold">{batchResult.skipped}</span></>
              )}
              {batchResult.failed > 0 && (
                <>, <span className="font-semibold text-red-600">{batchResult.failed} failed</span></>
              )}{" "}
              out of {batchResult.totalCounties} counties.
            </span>
            <button onClick={() => setBatchResult(null)} className="ml-2 font-medium hover:underline">
              Dismiss
            </button>
          </div>
        )}

        {/* Filters */}
        <div className="flex gap-3 mb-4">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search counties..."
            className="px-3 py-2 border border-border rounded-lg text-sm bg-background focus:outline-none focus:ring-2 focus:ring-ring w-56"
          />
          <div className="flex rounded-lg border border-border overflow-hidden">
            {STATUS_FILTERS.map((s) => (
              <button
                key={s.value}
                onClick={() => setStatusFilter(s.value)}
                className={`px-3 py-2 text-sm font-medium transition-colors ${
                  statusFilter === s.value
                    ? "bg-accent text-accent-foreground"
                    : "bg-background text-muted-foreground hover:bg-secondary"
                }`}
              >
                {s.label}
              </button>
            ))}
          </div>
        </div>

        {/* Stale toggle — independent of editorial status filter */}
        <div className="flex items-center gap-2 mb-4">
          <Switch
            id="stale-toggle"
            checked={onlyStale}
            onCheckedChange={setOnlyStale}
          />
          <Label htmlFor="stale-toggle" className="text-sm text-muted-foreground cursor-pointer">
            Only stale counties
            <span className="text-xs ml-1">({staleCount})</span>
          </Label>
        </div>

        {/* Error */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4">
            Error: {error.message}
          </div>
        )}

        {/* Table */}
        <div className="bg-card rounded-lg shadow-sm border border-border overflow-hidden">
          <table className="min-w-full divide-y divide-border">
            <thead className="bg-secondary">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  County
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Live Edition
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Draft Edition
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Status
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-muted-foreground uppercase tracking-wider">
                  Rows
                </th>
                <th className="w-10" />
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {filteredRows.map((row) => {
                const ed = row.currentEdition;
                return (
                  <tr
                    key={row.county.id}
                    onClick={() => {
                      if (ed) router.push(`/admin/editions/${ed.id}`);
                    }}
                    className={`transition-colors ${
                      ed ? "hover:bg-secondary cursor-pointer" : ""
                    }`}
                  >
                    <td className="px-6 py-3 whitespace-nowrap">
                      <span className="font-medium text-foreground">{row.county.name}</span>
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap text-sm">
                      <span className="inline-flex items-center gap-1.5">
                        <FreshnessIndicator isStale={row.isStale} status={ed?.status} />
                        <span className={row.isStale ? "text-muted-foreground" : "text-foreground"}>
                          {liveEditionAge(ed?.periodStart, row.isStale)}
                        </span>
                      </span>
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap text-sm text-muted-foreground">
                      {ed ? formatPeriod(ed.periodStart, ed.periodEnd) : "—"}
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap">
                      {ed ? (
                        <span
                          className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                            STATUS_BADGE_STYLES[ed.status] || "bg-secondary text-muted-foreground"
                          }`}
                        >
                          {STATUS_LABELS[ed.status] || ed.status}
                        </span>
                      ) : (
                        <span className="text-muted-foreground text-sm">—</span>
                      )}
                    </td>
                    <td className="px-6 py-3 whitespace-nowrap text-sm text-muted-foreground">
                      {ed ? `${ed.rowCount}` : "—"}
                    </td>
                    <td className="px-3 py-3 whitespace-nowrap">
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setHistoryCountyId(row.county.id);
                          setHistoryCountyName(row.county.name);
                        }}
                        className="p-1 text-muted-foreground hover:text-foreground rounded"
                        title="View edition history"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {filteredRows.length === 0 && (
            <div className="text-muted-foreground text-center py-12">
              {searchQuery || statusFilter
                ? "No counties match your filters."
                : "No counties found. Check your database."}
            </div>
          )}
        </div>
        <p className="text-xs text-muted-foreground mt-2 text-right">
          {filteredRows.length} of {rows.length} counties
        </p>
      </div>

      {/* ── Generate Drafts Confirmation Modal ─────────────────────────── */}
      <Dialog open={showGenerateModal} onOpenChange={setShowGenerateModal}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Generate Drafts</DialogTitle>
          </DialogHeader>
          <div className="space-y-3 text-sm">
            <p className="text-muted-foreground">
              This will generate broadsheet drafts for the current week across all counties:
            </p>
            <ul className="space-y-1.5">
              {generatePreview.toCreate > 0 && (
                <li className="flex items-center gap-2">
                  <span className="text-green-600 font-medium text-base">+</span>
                  <span><strong>{generatePreview.toCreate}</strong> new {generatePreview.toCreate === 1 ? "draft" : "drafts"} will be created</span>
                </li>
              )}
              {generatePreview.toRegenerate > 0 && (
                <li className="flex items-center gap-2">
                  <span className="text-amber-600 font-medium text-base">↻</span>
                  <span><strong>{generatePreview.toRegenerate}</strong> existing {generatePreview.toRegenerate === 1 ? "draft" : "drafts"} will be regenerated</span>
                </li>
              )}
              {generatePreview.toSkip > 0 && (
                <li className="flex items-center gap-2 text-muted-foreground">
                  <span className="font-medium text-base">—</span>
                  <span>
                    <strong>{generatePreview.toSkip}</strong> {generatePreview.toSkip === 1 ? "county" : "counties"} skipped
                    {[
                      generatePreview.skipReasons.reviewing > 0 && `${generatePreview.skipReasons.reviewing} reviewing`,
                      generatePreview.skipReasons.approved > 0 && `${generatePreview.skipReasons.approved} approved`,
                      generatePreview.skipReasons.published > 0 && `${generatePreview.skipReasons.published} published`,
                    ].filter(Boolean).length > 0 && (
                      <> ({[
                        generatePreview.skipReasons.reviewing > 0 && `${generatePreview.skipReasons.reviewing} reviewing`,
                        generatePreview.skipReasons.approved > 0 && `${generatePreview.skipReasons.approved} approved`,
                        generatePreview.skipReasons.published > 0 && `${generatePreview.skipReasons.published} published`,
                      ].filter(Boolean).join(", ")})</>
                    )}
                  </span>
                </li>
              )}
              {generatePreview.toCreate === 0 && generatePreview.toRegenerate === 0 && (
                <li className="text-muted-foreground">
                  Nothing to generate — all counties already have non-draft editions for this week.
                </li>
              )}
            </ul>
          </div>
          <div className="flex justify-end gap-3 mt-2">
            <button
              onClick={() => setShowGenerateModal(false)}
              className="px-3 py-2 text-sm rounded-lg border border-border hover:bg-secondary transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={() => {
                setShowGenerateModal(false);
                handleGenerate();
              }}
              disabled={generatePreview.toCreate === 0 && generatePreview.toRegenerate === 0}
              className="px-4 py-2 text-sm rounded-lg font-medium bg-admin-accent text-white hover:bg-admin-accent-hover disabled:opacity-50 transition-colors"
            >
              Generate
            </button>
          </div>
        </DialogContent>
      </Dialog>

      {/* ── Edition History Modal ──────────────────────────────────────── */}
      <Dialog
        open={!!historyCountyId}
        onOpenChange={(open) => { if (!open) setHistoryCountyId(null); }}
      >
        <DialogContent className="max-w-lg max-h-[70vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{historyCountyName} County — Edition History</DialogTitle>
          </DialogHeader>
          {historyFetching ? (
            <div className="py-8 text-center text-muted-foreground">Loading...</div>
          ) : (
            <div className="space-y-2">
              {(historyData?.editions?.editions || []).map((ed) => (
                <button
                  key={ed.id}
                  onClick={() => {
                    setHistoryCountyId(null);
                    router.push(`/admin/editions/${ed.id}`);
                  }}
                  className="w-full flex items-center justify-between px-4 py-3 rounded-lg border border-border hover:bg-secondary transition-colors text-left"
                >
                  <div>
                    <span className="text-sm font-medium text-foreground">
                      {formatPeriod(ed.periodStart, ed.periodEnd)}
                    </span>
                    <span className="text-xs text-muted-foreground ml-2">
                      {ed.rows.length} row{ed.rows.length !== 1 ? "s" : ""}
                    </span>
                  </div>
                  <span
                    className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                      STATUS_BADGE_STYLES[ed.status] || "bg-secondary text-muted-foreground"
                    }`}
                  >
                    {STATUS_LABELS[ed.status] || ed.status}
                  </span>
                </button>
              ))}
              {(historyData?.editions?.editions || []).length === 0 && (
                <p className="text-muted-foreground text-sm text-center py-4">
                  No editions found for this county.
                </p>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
