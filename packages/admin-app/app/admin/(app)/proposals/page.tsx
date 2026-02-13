"use client";

import { useState } from "react";
import Link from "next/link";
import { useRestate, callService, callObject, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { SyncBatch, SyncProposal } from "@/lib/restate/types";

type StatusFilter = "pending" | "all";
type ScoreFilter = "all" | "high" | "review" | "noise" | "unscored";

function StatusBadge({ status }: { status: string }) {
  const colors: Record<string, string> = {
    pending: "bg-yellow-100 text-yellow-800",
    partially_reviewed: "bg-blue-100 text-blue-800",
    approved: "bg-green-100 text-green-800",
    rejected: "bg-red-100 text-red-800",
    completed: "bg-stone-100 text-stone-800",
    expired: "bg-stone-100 text-stone-500",
  };
  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${colors[status] || "bg-stone-100 text-stone-600"}`}
    >
      {status?.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase()) || "Unknown"}
    </span>
  );
}

function OperationBadge({ operation }: { operation: string }) {
  const colors: Record<string, string> = {
    insert: "bg-blue-100 text-blue-800",
    update: "bg-amber-100 text-amber-800",
    delete: "bg-red-100 text-red-800",
    merge: "bg-purple-100 text-purple-800",
  };
  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${colors[operation] || "bg-stone-100 text-stone-600"}`}
    >
      {operation}
    </span>
  );
}

function ScoreBadge({ score }: { score: number | null | undefined }) {
  if (score == null) return null;
  const colors =
    score >= 8
      ? "bg-green-100 text-green-800"
      : score >= 5
        ? "bg-amber-100 text-amber-800"
        : "bg-red-100 text-red-800";
  return (
    <span
      className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-bold ${colors}`}
      title={`Relevance score: ${score}/10`}
    >
      {score}
    </span>
  );
}

function ConfidenceBadge({ confidence }: { confidence: string | null | undefined }) {
  if (!confidence) return null;
  const colors: Record<string, string> = {
    high: "bg-green-100 text-green-700",
    medium: "bg-yellow-100 text-yellow-700",
    low: "bg-orange-100 text-orange-700",
  };
  return (
    <span
      className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium ${colors[confidence] || "bg-stone-100 text-stone-600"}`}
      title={`AI confidence: ${confidence}`}
    >
      {confidence}
    </span>
  );
}

function EntityTypeBadge({ entityType }: { entityType: string }) {
  if (entityType === "post") return null;
  const colors: Record<string, string> = {
    note: "bg-indigo-100 text-indigo-800",
  };
  return (
    <span className={`inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium ${colors[entityType] || "bg-stone-100 text-stone-600"}`}>
      {entityType}
    </span>
  );
}

function PostLink({ id, label }: { id: string | null; label: string }) {
  if (!id) return <span>{label}</span>;
  return (
    <Link
      href={`/admin/posts/${id}`}
      onClick={(e) => e.stopPropagation()}
      className="underline decoration-stone-300 hover:decoration-stone-600 hover:text-stone-900 transition-colors"
    >
      {label}
    </Link>
  );
}

function ProposalDescription({ proposal: p }: { proposal: SyncProposal }) {
  const targetLabel = p.target_title || (p.target_entity_id ? `ID: ${p.target_entity_id.slice(0, 8)}...` : "untitled");
  const draftLabel = p.draft_title || (p.draft_entity_id ? `ID: ${p.draft_entity_id.slice(0, 8)}...` : null);

  switch (p.operation) {
    case "insert":
      return (
        <div className="mt-1">
          <p className="text-sm text-stone-800">
            <span className="font-medium">New {p.entity_type}:</span>{" "}
            {p.entity_type === "note" ? (
              <span>{draftLabel || "untitled note"}</span>
            ) : (
              <PostLink id={p.draft_entity_id} label={draftLabel || "untitled"} />
            )}
          </p>
        </div>
      );
    case "update":
      return (
        <div className="mt-1">
          <p className="text-sm text-stone-800">
            <span className="font-medium">Update:</span>{" "}
            <PostLink id={p.target_entity_id} label={targetLabel} />
          </p>
          {p.draft_title && p.draft_title !== p.target_title && (
            <p className="text-xs text-stone-500 mt-0.5">
              Revision: {p.draft_entity_id ? <PostLink id={p.draft_entity_id} label={p.draft_title} /> : p.draft_title}
            </p>
          )}
        </div>
      );
    case "delete":
      return (
        <div className="mt-1">
          <p className="text-sm text-stone-800">
            <span className="font-medium">Delete:</span>{" "}
            <PostLink id={p.target_entity_id} label={targetLabel} />
          </p>
        </div>
      );
    case "merge": {
      const sourceIds = p.merge_source_ids || [];
      const sourceTitles = p.merge_source_titles || [];
      const sources = sourceIds.map((id, i) => ({
        id,
        label: sourceTitles[i] || `ID: ${id.slice(0, 8)}...`,
      }));
      return (
        <div className="mt-1">
          <p className="text-sm text-stone-800">
            <span className="font-medium">Merge into:</span>{" "}
            <PostLink id={p.target_entity_id} label={targetLabel} />
          </p>
          {sources.length > 0 && (
            <p className="text-xs text-stone-600 mt-0.5">
              Absorbing:{" "}
              {sources.map((s, i) => (
                <span key={s.id}>
                  {i > 0 && ", "}
                  <PostLink id={s.id} label={s.label} />
                </span>
              ))}
            </p>
          )}
          {draftLabel && (
            <p className="text-xs text-stone-500 mt-0.5">
              Merged revision: <PostLink id={p.draft_entity_id} label={draftLabel} />
            </p>
          )}
        </div>
      );
    }
    default:
      return (
        <p className="text-sm text-stone-800 mt-1">
          {draftLabel || targetLabel || p.operation}
        </p>
      );
  }
}

function timeAgo(dateStr: string | null | undefined): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  if (isNaN(date.getTime())) return "";
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

function ProposalComment({
  proposalId,
  onSubmitted,
}: {
  proposalId: string;
  onSubmitted: () => void;
}) {
  const [comment, setComment] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    if (!comment.trim() || submitting) return;
    setSubmitting(true);
    try {
      const workflowId = `refine-${proposalId}-${Date.now()}`;
      await callObject("RefineProposalWorkflow", workflowId, "run", {
        proposal_id: proposalId,
        comment: comment.trim(),
        author_id: "00000000-0000-0000-0000-000000000000", // TODO: get from auth context
      });
      setComment("");
      invalidateService("Sync");
      onSubmitted();
    } catch (err) {
      console.error("Failed to submit comment:", err);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="flex gap-2 mt-2">
      <input
        type="text"
        value={comment}
        onChange={(e) => setComment(e.target.value)}
        placeholder="Comment for AI refinement..."
        className="flex-1 px-2 py-1 text-xs border border-stone-200 rounded bg-white focus:outline-none focus:ring-1 focus:ring-amber-400"
        onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
        disabled={submitting}
      />
      <button
        onClick={handleSubmit}
        disabled={!comment.trim() || submitting}
        className="px-2 py-1 text-xs font-medium bg-amber-100 text-amber-800 rounded hover:bg-amber-200 disabled:opacity-50"
      >
        {submitting ? "Refining..." : "Refine"}
      </button>
    </div>
  );
}

function BatchProposals({
  batchId,
  batchStatus,
  scoreFilter = "all",
}: {
  batchId: string;
  batchStatus: string;
  scoreFilter?: ScoreFilter;
}) {
  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [expandedReasoning, setExpandedReasoning] = useState<Set<string>>(new Set());

  const { data, isLoading, mutate } = useRestate<{ proposals: SyncProposal[] }>(
    "Sync", "list_proposals", { batch_id: batchId }, { revalidateOnFocus: false, keepPreviousData: true }
  );

  const allProposals = data?.proposals || [];
  const proposals = allProposals.filter((p) =>
    matchesScoreFilter(p.relevance_score, scoreFilter)
  );

  const toggleReasoning = (id: string) => {
    setExpandedReasoning((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const [actionError, setActionError] = useState<string | null>(null);

  const handleAction = async (
    proposalId: string,
    action: "approve" | "reject"
  ) => {
    setActionLoading(proposalId);
    setActionError(null);
    try {
      await callService("Sync", `${action}_proposal`, { proposal_id: proposalId });
      invalidateService("Sync");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setActionError(`Failed to ${action} proposal: ${message}`);
      console.error(`Failed to ${action} proposal:`, err);
    } finally {
      setActionLoading(null);
    }
  };

  if (isLoading && proposals.length === 0) return <AdminLoader />;
  if (!isLoading && proposals.length === 0) {
    return (
      <p className="text-sm text-stone-500 py-2">No proposals in this batch</p>
    );
  }

  return (
    <div className="space-y-2">
      {actionError && (
        <div className="px-3 py-2 bg-red-50 border border-red-200 rounded text-sm text-red-700">
          {actionError}
        </div>
      )}
      {proposals.map((p) => (
        <div
          key={p.id}
          className="p-3 bg-stone-50 rounded-lg border border-stone-100"
        >
          <div className="flex items-start justify-between gap-4">
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 flex-wrap">
                <OperationBadge operation={p.operation} />
                <EntityTypeBadge entityType={p.entity_type} />
                <StatusBadge status={p.status} />
                <ScoreBadge score={p.relevance_score} />
                <ConfidenceBadge confidence={p.confidence} />
                {(p.revision_count ?? 0) > 0 && (
                  <span className="inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-violet-100 text-violet-700" title={`Revised ${p.revision_count} time(s)`}>
                    rev {p.revision_count}
                  </span>
                )}
              </div>
              <ProposalDescription proposal={p} />

              {/* Source URLs */}
              {p.source_urls && p.source_urls.length > 0 && (
                <div className="flex flex-wrap gap-1.5 mt-1.5">
                  {p.source_urls.map((url, i) => (
                    <a
                      key={i}
                      href={url}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={(e) => e.stopPropagation()}
                      className="text-xs text-blue-600 hover:text-blue-800 underline decoration-blue-300 hover:decoration-blue-500"
                    >
                      {new URL(url).hostname.replace("www.", "")}{"\u2197"}
                    </a>
                  ))}
                </div>
              )}

              {/* Curator reasoning (expandable) */}
              {p.curator_reasoning && (
                <div className="mt-2">
                  <button
                    onClick={() => toggleReasoning(p.id)}
                    className="text-xs text-stone-400 hover:text-stone-600 transition-colors"
                  >
                    {expandedReasoning.has(p.id) ? "\u25B2 Hide reasoning" : "\u25BC AI reasoning"}
                  </button>
                  {expandedReasoning.has(p.id) && (
                    <p className="text-xs text-stone-600 mt-1 leading-relaxed bg-stone-100 rounded px-2 py-1.5">
                      {p.curator_reasoning}
                    </p>
                  )}
                </div>
              )}

              {/* Legacy reason field (non-curator proposals) */}
              {!p.curator_reasoning && p.reason && (
                <p className="text-xs text-stone-500 mt-2 leading-relaxed bg-stone-100 rounded px-2 py-1">
                  {p.reason}
                </p>
              )}

              {/* Direct edit link */}
              {p.status === "pending" && p.draft_entity_id && p.entity_type === "post" && (
                <div className="mt-1.5">
                  <Link
                    href={`/admin/posts/${p.draft_entity_id}`}
                    onClick={(e) => e.stopPropagation()}
                    className="text-xs text-amber-600 hover:text-amber-800 font-medium"
                  >
                    Edit draft {"\u2192"}
                  </Link>
                </div>
              )}

              {/* Comment input for refinement */}
              {p.status === "pending" && batchStatus !== "expired" && (
                <ProposalComment proposalId={p.id} onSubmitted={() => mutate()} />
              )}
            </div>
            {p.status === "pending" && batchStatus !== "expired" && (
              <div className="flex gap-1 shrink-0">
                <button
                  onClick={() => handleAction(p.id, "approve")}
                  disabled={actionLoading === p.id}
                  className="px-2.5 py-1 text-xs font-medium bg-emerald-400 text-white rounded hover:bg-emerald-500 disabled:opacity-50"
                >
                  {actionLoading === p.id ? "..." : "Approve"}
                </button>
                <button
                  onClick={() => handleAction(p.id, "reject")}
                  disabled={actionLoading === p.id}
                  className="px-2.5 py-1 text-xs font-medium bg-rose-400 text-white rounded hover:bg-rose-500 disabled:opacity-50"
                >
                  {actionLoading === p.id ? "..." : "Reject"}
                </button>
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}

function BatchCard({ batch, expanded, onToggle, scoreFilter = "all" }: { batch: SyncBatch; expanded: boolean; onToggle: () => void; scoreFilter?: ScoreFilter }) {
  const [batchActionLoading, setBatchActionLoading] = useState(false);

  const proposalCount = batch.proposal_count || 0;
  const approvedCount = batch.approved_count || 0;
  const rejectedCount = batch.rejected_count || 0;
  const pendingCount = proposalCount - approvedCount - rejectedCount;
  const hasPending =
    pendingCount > 0 &&
    batch.status !== "expired" &&
    batch.status !== "completed";

  const [batchError, setBatchError] = useState<string | null>(null);

  const handleBatchAction = async (action: "approve" | "reject") => {
    setBatchActionLoading(true);
    setBatchError(null);
    try {
      await callService("Sync", `${action}_batch`, { batch_id: batch.id });
      invalidateService("Sync");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setBatchError(`Failed to ${action} batch: ${message}`);
      console.error(`Failed to ${action} batch:`, err);
    } finally {
      setBatchActionLoading(false);
    }
  };

  const createdAgo = timeAgo(batch.created_at);

  return (
    <div className="bg-white border border-stone-200 rounded-lg overflow-hidden">
      <div
        className="p-4 cursor-pointer hover:bg-stone-50 transition-colors"
        onClick={onToggle}
      >
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <StatusBadge status={batch.status} />
              {batch.source_name && (
                <span className="text-xs font-medium text-stone-700">
                  {batch.source_name}
                </span>
              )}
              {createdAgo && (
                <span className="text-xs text-stone-400">
                  {createdAgo}
                </span>
              )}
            </div>
            {batch.summary && (
              <p className="text-sm text-stone-700 mt-1">{batch.summary}</p>
            )}
            <div className="flex items-center gap-3 mt-2 text-xs text-stone-500">
              {proposalCount === 0 ? (
                <span className="text-stone-400 italic">
                  No actionable proposals
                </span>
              ) : (
                <>
                  <span>{proposalCount} proposals</span>
                  {approvedCount > 0 && (
                    <span className="text-green-600">
                      {approvedCount} approved
                    </span>
                  )}
                  {rejectedCount > 0 && (
                    <span className="text-red-600">
                      {rejectedCount} rejected
                    </span>
                  )}
                  {pendingCount > 0 && (
                    <span className="text-yellow-600">
                      {pendingCount} pending
                    </span>
                  )}
                </>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {hasPending ? (
              <>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleBatchAction("approve");
                  }}
                  disabled={batchActionLoading}
                  className="px-3 py-1.5 text-xs font-medium bg-emerald-400 text-white rounded hover:bg-emerald-500 disabled:opacity-50"
                >
                  {batchActionLoading ? "..." : "Approve All"}
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleBatchAction("reject");
                  }}
                  disabled={batchActionLoading}
                  className="px-3 py-1.5 text-xs font-medium bg-rose-400 text-white rounded hover:bg-rose-500 disabled:opacity-50"
                >
                  {batchActionLoading ? "..." : "Reject All"}
                </button>
              </>
            ) : batch.status !== "completed" && batch.status !== "expired" && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleBatchAction("approve");
                }}
                disabled={batchActionLoading}
                className="px-3 py-1.5 text-xs font-medium bg-stone-400 text-white rounded hover:bg-stone-500 disabled:opacity-50"
              >
                {batchActionLoading ? "..." : "Dismiss"}
              </button>
            )}
            <span className="text-stone-400 text-sm">
              {expanded ? "\u25B2" : "\u25BC"}
            </span>
          </div>
        </div>
      </div>
      {batchError && (
        <div className="mx-4 mb-3 px-3 py-2 bg-red-50 border border-red-200 rounded text-sm text-red-700">
          {batchError}
        </div>
      )}
      {expanded && (
        <div className="border-t border-stone-100 p-4">
          <BatchProposals batchId={batch.id} batchStatus={batch.status} scoreFilter={scoreFilter} />
        </div>
      )}
    </div>
  );
}

function matchesScoreFilter(score: number | null | undefined, filter: ScoreFilter): boolean {
  if (filter === "all") return true;
  if (filter === "unscored") return score == null;
  if (score == null) return false;
  if (filter === "high") return score >= 8;
  if (filter === "review") return score >= 5 && score <= 7;
  if (filter === "noise") return score <= 4;
  return true;
}

export default function ProposalsPage() {
  const [filter, setFilter] = useState<StatusFilter>("pending");
  const [scoreFilter, setScoreFilter] = useState<ScoreFilter>("all");
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const toggleExpanded = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const variables =
    filter === "pending"
      ? { status: "pending", limit: 50 }
      : { limit: 50 };

  const { data, isLoading } = useRestate<{ batches: SyncBatch[] }>(
    "Sync", "list_batches", variables, { revalidateOnFocus: false, keepPreviousData: true }
  );

  const batches = data?.batches || [];

  const pendingCount = batches.filter(
    (b) => b.status === "pending" || b.status === "partially_reviewed"
  ).length;

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-stone-900">AI Proposals</h1>
          <p className="text-sm text-stone-600 mt-1">
            Review AI-proposed changes before they go live. Each batch contains
            proposals from a single sync operation.
          </p>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="bg-white border border-stone-200 rounded-lg p-4">
            <div className="text-2xl font-bold text-stone-900">
              {pendingCount}
            </div>
            <div className="text-xs text-stone-500">Pending batches</div>
          </div>
          <div className="bg-white border border-stone-200 rounded-lg p-4">
            <div className="text-2xl font-bold text-stone-900">
              {batches.length}
            </div>
            <div className="text-xs text-stone-500">Total batches shown</div>
          </div>
          <div className="bg-white border border-stone-200 rounded-lg p-4">
            <div className="text-2xl font-bold text-stone-900">
              {batches.reduce((sum, b) => sum + (b.proposal_count || 0), 0)}
            </div>
            <div className="text-xs text-stone-500">Total proposals</div>
          </div>
        </div>

        {/* Filter tabs */}
        <div className="flex items-center gap-4 mb-4">
          <div className="flex gap-1">
            <button
              onClick={() => setFilter("pending")}
              className={`px-3 py-1.5 text-sm font-medium rounded ${
                filter === "pending"
                  ? "bg-amber-100 text-amber-800"
                  : "text-stone-600 hover:bg-stone-100"
              }`}
            >
              Pending Review
            </button>
            <button
              onClick={() => setFilter("all")}
              className={`px-3 py-1.5 text-sm font-medium rounded ${
                filter === "all"
                  ? "bg-amber-100 text-amber-800"
                  : "text-stone-600 hover:bg-stone-100"
              }`}
            >
              All Batches
            </button>
          </div>
          <select
            value={scoreFilter}
            onChange={(e) => setScoreFilter(e.target.value as ScoreFilter)}
            className="px-2 py-1.5 text-sm border border-stone-300 rounded bg-white text-stone-700"
          >
            <option value="all">All scores</option>
            <option value="high">High (8-10)</option>
            <option value="review">Review (5-7)</option>
            <option value="noise">Noise (1-4)</option>
            <option value="unscored">Unscored</option>
          </select>
        </div>

        {/* Batch list */}
        {isLoading && batches.length === 0 ? (
          <AdminLoader />
        ) : batches.length === 0 ? (
          <div className="bg-white border border-stone-200 rounded-lg p-12 text-center">
            <p className="text-stone-500">
              {filter === "pending"
                ? "No pending proposals. The waiting room is empty."
                : "No batches found."}
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {batches.map((batch) => (
              <BatchCard
                key={batch.id}
                batch={batch}
                expanded={expandedIds.has(batch.id)}
                onToggle={() => toggleExpanded(batch.id)}
                scoreFilter={scoreFilter}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
