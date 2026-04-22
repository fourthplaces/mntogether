"use client";

import { useState, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  CountyDashboardQuery,
  EditionHistoryQuery,
  BatchGenerateEditionsMutation,
  UpdateCountyTargetContentWeightMutation,
} from "@/lib/graphql/editions";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";
import {
  AlertTriangle,
  CheckCircle2,
  Circle,
  CircleDashed,
  CircleDot,
  Clock,
  Minus,
} from "lucide-react";

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
  archived: "bg-muted text-muted-foreground",
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
  // One Lucide icon per state; no unicode glyphs. `title` still
  // carries the human-readable label for hover.
  const iconProps = { className: "size-4", "aria-hidden": true } as const;

  if (!status) {
    return (
      <span className="text-muted-foreground inline-flex" title="No edition">
        <Minus {...iconProps} />
      </span>
    );
  }
  if (status === "published" && !isStale) {
    return (
      <span className="text-green-600 inline-flex" title="Published (current)">
        <CheckCircle2 {...iconProps} />
      </span>
    );
  }
  if (isStale) {
    return (
      <span className="text-amber-600 inline-flex" title="Stale">
        <AlertTriangle {...iconProps} />
      </span>
    );
  }
  if (status === "approved") {
    return (
      <span className="text-emerald-600 inline-flex" title="Approved">
        <CircleDot {...iconProps} />
      </span>
    );
  }
  if (status === "in_review") {
    return (
      <span className="text-amber-500 inline-flex" title="Reviewing">
        <CircleDashed {...iconProps} />
      </span>
    );
  }
  return (
    <span className="text-muted-foreground inline-flex" title="Draft">
      <Circle {...iconProps} />
    </span>
  );
}

// ─── Status pill ─────────────────────────────────────────────────────────────

const WORKFLOW_STEPS = [
  { key: "draft", label: "Draft" },
  { key: "in_review", label: "Reviewing" },
  { key: "approved", label: "Approved" },
  { key: "published", label: "Published" },
] as const;

const STEP_ACTIVE_STYLE: Record<string, string> = {
  draft: "bg-yellow-100 text-yellow-800 border-yellow-200",
  in_review: "bg-amber-100 text-amber-800 border-amber-200",
  approved: "bg-emerald-100 text-emerald-800 border-emerald-200",
  published: "bg-green-100 text-green-800 border-green-200",
};

function StatusPill({ status }: { status: string }) {
  // Approved editions are waiting to go live — show the 3-step workflow
  // Published editions are live — collapse approved+published into "Published"
  const steps = status === "published"
    ? WORKFLOW_STEPS.filter((s) => s.key !== "approved")
    : WORKFLOW_STEPS.filter((s) => s.key !== "published");
  const active = status;

  return (
    <span className="inline-flex text-[11px] font-medium leading-none">
      {steps.map((step, i) => (
        <span
          key={step.key}
          className={`px-2 py-1 border ${
            i === 0 ? "rounded-l-full" : "-ml-px"
          } ${
            i === steps.length - 1 ? "rounded-r-full" : ""
          } ${
            step.key === active
              ? STEP_ACTIVE_STYLE[step.key]
              : "bg-muted/50 text-muted-foreground border-border"
          }`}
        >
          {step.label}
        </span>
      ))}
    </span>
  );
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

  // Target content weight editing
  const [targetDialogCounty, setTargetDialogCounty] = useState<{
    id: string;
    name: string;
    value: number;
  } | null>(null);
  const [targetDraft, setTargetDraft] = useState<string>("");
  const [, updateTargetWeight] = useMutation(UpdateCountyTargetContentWeightMutation);

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
          <Button
            variant="admin"
            onClick={() => setShowGenerateModal(true)}
            loading={batching}
          >
            Generate Drafts
          </Button>
        </div>

        {/* Batch result / error */}
        {batchError && (
          <Alert variant="error" className="mb-4 flex items-center justify-between">
            {batchError}
            <Button variant="ghost" size="xs" onClick={() => setBatchError(null)}>
              Dismiss
            </Button>
          </Alert>
        )}
        {batchResult && (
          <Alert variant="success" className="mb-4 flex items-center justify-between">
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
            <Button variant="ghost" size="xs" onClick={() => setBatchResult(null)}>
              Dismiss
            </Button>
          </Alert>
        )}

        {/* Filters */}
        <div className="flex items-center gap-3 mb-4">
          <Tabs value={statusFilter} onValueChange={setStatusFilter}>
            <TabsList>
              {STATUS_FILTERS.map((s) => (
                <TabsTrigger key={s.value} value={s.value}>
                  {s.label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          <Tabs value={onlyStale ? "stale" : "all"} onValueChange={(v) => setOnlyStale(v === "stale")}>
            <TabsList>
              <TabsTrigger value="all">All</TabsTrigger>
              <TabsTrigger value="stale">Stale ({staleCount})</TabsTrigger>
            </TabsList>
          </Tabs>

          <Input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search counties..."
            className="flex-1"
          />
        </div>

        {/* Error */}
        {error && (
          <Alert variant="error" className="mb-4">
            Error: {error.message}
          </Alert>
        )}

        {/* Table */}
        <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="pl-6">County</TableHead>
                <TableHead>Live Edition</TableHead>
                <TableHead>Draft Edition</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="w-20">Rows</TableHead>
                <TableHead className="w-24" title="Target editorial weight per edition (heavy=3, medium=2, light=1)">Target</TableHead>
                <TableHead className="w-10" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredRows.map((row) => {
                const ed = row.currentEdition;
                return (
                  <TableRow
                    key={row.county.id}
                    onClick={() => {
                      if (ed) router.push(`/admin/editions/${ed.id}`);
                    }}
                    className={ed ? "cursor-pointer" : ""}
                  >
                    <TableCell className="pl-6 whitespace-nowrap">
                      <span className="font-medium text-foreground">{row.county.name}</span>
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <span className="inline-flex items-center gap-1.5">
                        <FreshnessIndicator isStale={row.isStale} status={ed?.status} />
                        <span className={row.isStale ? "text-muted-foreground" : "text-foreground"}>
                          {liveEditionAge(ed?.periodStart, row.isStale)}
                        </span>
                      </span>
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-muted-foreground">
                      {ed ? formatPeriod(ed.periodStart, ed.periodEnd) : "—"}
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      {ed ? (
                        <StatusPill status={ed.status} />
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell className="whitespace-nowrap text-muted-foreground">
                      {ed ? `${ed.rowCount}` : "—"}
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-7 px-2 font-mono text-xs"
                        onClick={(e) => {
                          e.stopPropagation();
                          setTargetDialogCounty({
                            id: row.county.id,
                            name: row.county.name,
                            value: row.county.targetContentWeight,
                          });
                          setTargetDraft(String(row.county.targetContentWeight));
                        }}
                        title="Edit editorial weight target (heavy=3, medium=2, light=1)"
                      >
                        {row.county.targetContentWeight}
                      </Button>
                    </TableCell>
                    <TableCell className="whitespace-nowrap">
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        onClick={(e) => {
                          e.stopPropagation();
                          setHistoryCountyId(row.county.id);
                          setHistoryCountyName(row.county.name);
                        }}
                        title="View edition history"
                      >
                        <Clock className="size-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        {filteredRows.length === 0 && (
          <div className="text-muted-foreground text-center py-12">
            {searchQuery || statusFilter
              ? "No counties match your filters."
              : "No counties found. Check your database."}
          </div>
        )}
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
            <Button
              variant="outline"
              onClick={() => setShowGenerateModal(false)}
            >
              Cancel
            </Button>
            <Button
              variant="admin"
              onClick={() => {
                setShowGenerateModal(false);
                handleGenerate();
              }}
              disabled={generatePreview.toCreate === 0 && generatePreview.toRegenerate === 0}
            >
              Generate
            </Button>
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
                <Button
                  key={ed.id}
                  variant="outline"
                  className="w-full h-auto flex items-center justify-between px-4 py-3 text-left"
                  onClick={() => {
                    setHistoryCountyId(null);
                    router.push(`/admin/editions/${ed.id}`);
                  }}
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
                </Button>
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

      {/* ── Target Content Weight Edit Dialog ──────────────────────────── */}
      <Dialog
        open={!!targetDialogCounty}
        onOpenChange={(open) => {
          if (!open) setTargetDialogCounty(null);
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>
              {targetDialogCounty?.name} County — Editorial Weight Target
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="text-sm text-muted-foreground leading-relaxed">
              Sum of post weights (Heavy = 3, Medium = 2, Light = 1) that
              Root Signal aims to produce per edition. The layout engine
              flexes up to ~1.3× on busy weeks and scales down when the pool
              is short. Typical: 40 posts of mixed weight ≈ 66 points.
            </div>
            <div className="flex items-center gap-3">
              <label className="text-sm font-medium">Target weight</label>
              <Input
                type="number"
                min={1}
                value={targetDraft}
                onChange={(e) => setTargetDraft(e.target.value)}
                className="w-32 font-mono"
                autoFocus
              />
            </div>
            <div className="flex justify-end gap-2 pt-2">
              <Button
                variant="ghost"
                onClick={() => setTargetDialogCounty(null)}
              >
                Cancel
              </Button>
              <Button
                onClick={async () => {
                  const n = parseInt(targetDraft, 10);
                  if (!Number.isFinite(n) || n < 1) return;
                  if (!targetDialogCounty) return;
                  await updateTargetWeight(
                    { id: targetDialogCounty.id, targetContentWeight: n },
                    { additionalTypenames: ["County", "CountyDashboardRow"] }
                  );
                  setTargetDialogCounty(null);
                }}
                disabled={
                  !targetDraft ||
                  !Number.isFinite(parseInt(targetDraft, 10)) ||
                  parseInt(targetDraft, 10) < 1
                }
              >
                Save
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
