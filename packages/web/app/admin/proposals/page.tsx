"use client";

import { useState } from "react";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { SyncBatch, SyncProposal } from "@/lib/restate/types";

type StatusFilter = "pending" | "all";

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
      {status.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())}
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

function ProposalDescription({ proposal: p }: { proposal: SyncProposal }) {
  switch (p.operation) {
    case "insert":
      return (
        <p className="text-sm text-stone-800 mt-1">
          <span className="font-medium">New post:</span>{" "}
          {p.draft_title || p.draft_entity_id?.slice(0, 8) || "unknown"}
        </p>
      );
    case "update":
      return (
        <p className="text-sm text-stone-800 mt-1">
          <span className="font-medium">Update:</span>{" "}
          {p.target_title || p.target_entity_id?.slice(0, 8) || "unknown"}
          {p.draft_title && p.draft_title !== p.target_title && (
            <span className="text-stone-500">
              {" "}
              (revision: {p.draft_title})
            </span>
          )}
        </p>
      );
    case "delete":
      return (
        <p className="text-sm text-stone-800 mt-1">
          <span className="font-medium">Delete:</span>{" "}
          {p.target_title || p.target_entity_id?.slice(0, 8) || "unknown"}
        </p>
      );
    case "merge":
      return (
        <div className="text-sm text-stone-800 mt-1">
          <p>
            <span className="font-medium">Merge into:</span>{" "}
            {p.target_title || p.target_entity_id?.slice(0, 8) || "unknown"}
          </p>
          {p.merge_source_titles.length > 0 && (
            <p className="text-stone-600 mt-0.5">
              Absorbing:{" "}
              {p.merge_source_titles.join(", ")}
            </p>
          )}
        </div>
      );
    default:
      return (
        <p className="text-sm text-stone-800 mt-1">
          {p.draft_title || p.target_title || p.operation}
        </p>
      );
  }
}

function timeAgo(dateStr: string): string {
  const date = new Date(dateStr);
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

function BatchProposals({
  batchId,
  batchStatus,
}: {
  batchId: string;
  batchStatus: string;
}) {
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const { data, isLoading } = useRestate<SyncProposal[]>(
    "Sync", "list_proposals", { batch_id: batchId }, { revalidateOnFocus: false }
  );

  const proposals = data || [];

  const handleAction = async (
    proposalId: string,
    action: "approve" | "reject"
  ) => {
    setActionLoading(proposalId);
    try {
      await callService("Sync", `${action}_proposal`, { proposal_id: proposalId });
      invalidateService("Sync");
    } catch (err) {
      alert(
        err instanceof Error ? err.message : `Failed to ${action} proposal`
      );
    } finally {
      setActionLoading(null);
    }
  };

  if (isLoading) return <AdminLoader />;
  if (proposals.length === 0) {
    return (
      <p className="text-sm text-stone-500 py-2">No proposals in this batch</p>
    );
  }

  return (
    <div className="space-y-2">
      {proposals.map((p) => (
        <div
          key={p.id}
          className="flex items-start justify-between gap-4 p-3 bg-stone-50 rounded-lg border border-stone-100"
        >
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <OperationBadge operation={p.operation} />
              <StatusBadge status={p.status} />
              <span className="text-xs text-stone-500">{p.entity_type}</span>
            </div>
            <ProposalDescription proposal={p} />
            {p.reason && (
              <p className="text-sm text-stone-600 mt-1 italic">{p.reason}</p>
            )}
          </div>
          {p.status === "pending" && batchStatus !== "expired" && (
            <div className="flex gap-1 shrink-0">
              <button
                onClick={() => handleAction(p.id, "approve")}
                disabled={actionLoading === p.id}
                className="px-2.5 py-1 text-xs font-medium bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
              >
                {actionLoading === p.id ? "..." : "Approve"}
              </button>
              <button
                onClick={() => handleAction(p.id, "reject")}
                disabled={actionLoading === p.id}
                className="px-2.5 py-1 text-xs font-medium bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
              >
                {actionLoading === p.id ? "..." : "Reject"}
              </button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

function BatchCard({ batch }: { batch: SyncBatch }) {
  const [expanded, setExpanded] = useState(false);
  const [batchActionLoading, setBatchActionLoading] = useState(false);

  const pendingCount =
    batch.proposal_count - batch.approved_count - batch.rejected_count;
  const hasPending =
    pendingCount > 0 &&
    batch.status !== "expired" &&
    batch.status !== "completed";

  const handleBatchAction = async (action: "approve" | "reject") => {
    setBatchActionLoading(true);
    try {
      await callService("Sync", `${action}_batch`, { batch_id: batch.id });
      invalidateService("Sync");
    } catch (err) {
      alert(
        err instanceof Error ? err.message : `Failed to ${action} batch`
      );
    } finally {
      setBatchActionLoading(false);
    }
  };

  return (
    <div className="bg-white border border-stone-200 rounded-lg overflow-hidden">
      <div
        className="p-4 cursor-pointer hover:bg-stone-50 transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <StatusBadge status={batch.status} />
              <span className="text-xs text-stone-500">
                {batch.resource_type}
              </span>
              <span className="text-xs text-stone-400">
                {timeAgo(batch.created_at)}
              </span>
            </div>
            {batch.summary && (
              <p className="text-sm text-stone-700 mt-1">{batch.summary}</p>
            )}
            <div className="flex items-center gap-3 mt-2 text-xs text-stone-500">
              {batch.proposal_count === 0 ? (
                <span className="text-stone-400 italic">
                  No actionable proposals (LLM output could not be staged)
                </span>
              ) : (
                <>
                  <span>{batch.proposal_count} proposals</span>
                  {batch.approved_count > 0 && (
                    <span className="text-green-600">
                      {batch.approved_count} approved
                    </span>
                  )}
                  {batch.rejected_count > 0 && (
                    <span className="text-red-600">
                      {batch.rejected_count} rejected
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
            {hasPending && (
              <>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleBatchAction("approve");
                  }}
                  disabled={batchActionLoading}
                  className="px-3 py-1.5 text-xs font-medium bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
                >
                  {batchActionLoading ? "..." : "Approve All"}
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleBatchAction("reject");
                  }}
                  disabled={batchActionLoading}
                  className="px-3 py-1.5 text-xs font-medium bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
                >
                  {batchActionLoading ? "..." : "Reject All"}
                </button>
              </>
            )}
            <span className="text-stone-400 text-sm">
              {expanded ? "\u25B2" : "\u25BC"}
            </span>
          </div>
        </div>
      </div>
      {expanded && (
        <div className="border-t border-stone-100 p-4">
          <BatchProposals batchId={batch.id} batchStatus={batch.status} />
        </div>
      )}
    </div>
  );
}

export default function ProposalsPage() {
  const [filter, setFilter] = useState<StatusFilter>("pending");

  const variables =
    filter === "pending"
      ? { status: "pending", limit: 50 }
      : { limit: 50 };

  const { data, isLoading } = useRestate<SyncBatch[]>(
    "Sync", "list_batches", variables, { revalidateOnFocus: false }
  );

  const batches = data || [];

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
              {batches.reduce((sum, b) => sum + b.proposal_count, 0)}
            </div>
            <div className="text-xs text-stone-500">Total proposals</div>
          </div>
        </div>

        {/* Filter tabs */}
        <div className="flex gap-1 mb-4">
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

        {/* Batch list */}
        {isLoading ? (
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
              <BatchCard key={batch.id} batch={batch} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
