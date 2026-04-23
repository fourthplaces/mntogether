"use client";

import { useMemo, useState } from "react";
import { useMutation, useQuery } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { InboxCard, type InboxCardPost } from "@/components/admin/InboxCard";
import { MergeDuplicateDialog } from "@/components/admin/MergeDuplicateDialog";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { ChevronDown, ChevronRight, Inbox } from "lucide-react";
import { SignalInboxQuery } from "@/lib/graphql/signal-inbox";
import {
  ApprovePostMutation,
  RejectPostMutation,
} from "@/lib/graphql/posts";

// ─── Flag taxonomy ──────────────────────────────────────────────────────────
//
// Groups in display order. Labels / summaries are derived from the Root
// Signal handoff spec §11.2 soft-failure conditions. Posts land under
// their *first* flag (primary reason) for grouping, but all flags show
// as badges on the card.

interface FlagGroupDef {
  key: string;
  label: string;
  summary: string;
}

const FLAG_GROUPS: FlagGroupDef[] = [
  {
    key: "low_confidence",
    label: "Low extraction confidence",
    summary: "Root Signal's extraction_confidence score was under 60.",
  },
  {
    key: "possible_duplicate",
    label: "Possible duplicate",
    summary: "Root Signal flagged this as a potential duplicate of an existing post.",
  },
  {
    key: "deck_missing_on_heavy",
    label: "Heavy post missing deck",
    summary: "A heavy-weight post was submitted without the required deck.",
  },
  {
    key: "individual_no_consent",
    label: "Individual source — consent withheld",
    summary: "Individual source submitted without consent to publish.",
  },
  {
    key: "source_stale",
    label: "Source metadata stale",
    summary: "Incoming organisation metadata diverged from the stored record.",
  },
  {
    key: "other",
    label: "Other / uncategorised",
    summary: "Flagged by Root Signal but doesn't match a known group.",
  },
];

function primaryFlag(flags: readonly string[]): string {
  for (const g of FLAG_GROUPS) {
    if (flags.includes(g.key)) return g.key;
  }
  return "other";
}

// ─── Component ──────────────────────────────────────────────────────────────

export default function SignalInboxPage() {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busyIds, setBusyIds] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [merging, setMerging] = useState<{
    incoming: InboxCardPost;
    canonicalId: string;
  } | null>(null);

  const [{ data, fetching, error: fetchError }, refetch] = useQuery({
    query: SignalInboxQuery,
    variables: { limit: 200, offset: 0 },
    requestPolicy: "cache-and-network",
  });

  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, rejectPost] = useMutation(RejectPostMutation);

  const rows = data?.signalInbox?.rows ?? [];
  const totalCount = data?.signalInbox?.totalCount ?? 0;

  const grouped = useMemo(() => {
    const map = new Map<string, typeof rows>();
    for (const g of FLAG_GROUPS) map.set(g.key, []);
    for (const row of rows) {
      const key = primaryFlag(row.reviewFlags);
      map.get(key)!.push(row);
    }
    return map;
  }, [rows]);

  const toggleCollapsed = (key: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectGroup = (groupKey: string, select: boolean) => {
    const groupRows = grouped.get(groupKey) ?? [];
    setSelected((prev) => {
      const next = new Set(prev);
      for (const r of groupRows) {
        if (select) next.add(r.post.id);
        else next.delete(r.post.id);
      }
      return next;
    });
  };

  const selectAll = (select: boolean) => {
    if (select) setSelected(new Set(rows.map((r) => r.post.id)));
    else setSelected(new Set());
  };

  const markBusy = (ids: string[], busy: boolean) => {
    setBusyIds((prev) => {
      const next = new Set(prev);
      for (const id of ids) {
        if (busy) next.add(id);
        else next.delete(id);
      }
      return next;
    });
  };

  const runApprove = async (ids: string[]) => {
    setError(null);
    markBusy(ids, true);
    try {
      for (const id of ids) {
        const res = await approvePost(
          { id },
          { additionalTypenames: ["Post", "PostConnection", "SignalInboxConnection"] }
        );
        if (res.error) throw res.error;
      }
      setSelected((prev) => {
        const next = new Set(prev);
        for (const id of ids) next.delete(id);
        return next;
      });
      refetch({ requestPolicy: "network-only" });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      markBusy(ids, false);
    }
  };

  const runReject = async (ids: string[], reason: string) => {
    setError(null);
    markBusy(ids, true);
    try {
      for (const id of ids) {
        const res = await rejectPost(
          { id, reason },
          { additionalTypenames: ["Post", "PostConnection", "SignalInboxConnection"] }
        );
        if (res.error) throw res.error;
      }
      setSelected((prev) => {
        const next = new Set(prev);
        for (const id of ids) next.delete(id);
        return next;
      });
      refetch({ requestPolicy: "network-only" });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      markBusy(ids, false);
    }
  };

  const handleApproveOne = (id: string) => runApprove([id]);

  const handleRejectOne = (id: string) => {
    if (!confirm("Reject this post? It will be marked rejected and removed from the queue.")) return;
    runReject([id], "Rejected from Signal Inbox");
  };

  const handleMerge = (row: (typeof rows)[number]) => {
    if (!row.post.duplicateOfId) return;
    setMerging({
      incoming: row.post as InboxCardPost,
      canonicalId: row.post.duplicateOfId,
    });
  };

  const handleConfirmMerge = async () => {
    if (!merging) return;
    await runReject(
      [merging.incoming.id],
      `Merged into canonical post ${merging.canonicalId}`
    );
    setMerging(null);
  };

  const handleBulkApprove = () => {
    if (selected.size === 0) return;
    if (!confirm(`Approve ${selected.size} post${selected.size === 1 ? "" : "s"}?`)) return;
    runApprove(Array.from(selected));
  };

  const handleBulkReject = () => {
    if (selected.size === 0) return;
    if (!confirm(`Reject ${selected.size} post${selected.size === 1 ? "" : "s"}?`)) return;
    runReject(Array.from(selected), "Rejected from Signal Inbox (bulk)");
  };

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="mb-5 flex items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2">
              <Inbox className="w-5 h-5 text-muted-foreground" />
              <h1 className="text-2xl font-bold text-foreground">Signal Inbox</h1>
            </div>
            <p className="text-muted-foreground text-sm mt-0.5">
              Posts Root Signal soft-failed for editor review. Approve, reject, or merge.
              {totalCount > 0 && (
                <> <span className="text-foreground font-medium">{totalCount}</span> awaiting triage.</>
              )}
            </p>
          </div>
        </div>

        {/* Bulk action bar */}
        {rows.length > 0 && (
          <div className="sticky top-0 z-10 bg-background/95 backdrop-blur border border-border rounded-lg px-3 py-2 mb-4 flex items-center gap-3 text-sm">
            <Checkbox
              checked={selected.size > 0 && selected.size === rows.length}
              onCheckedChange={(c) => selectAll(!!c)}
              aria-label="Select all"
            />
            <span className="text-muted-foreground">
              {selected.size > 0
                ? `${selected.size} selected`
                : `Select all (${rows.length})`}
            </span>
            <div className="ml-auto flex gap-2">
              <Button
                variant="admin"
                size="sm"
                disabled={selected.size === 0}
                onClick={handleBulkApprove}
              >
                Approve selected
              </Button>
              <Button
                variant="outline"
                size="sm"
                disabled={selected.size === 0}
                onClick={handleBulkReject}
                className="text-red-700"
              >
                Reject selected
              </Button>
            </div>
          </div>
        )}

        {error && (
          <Alert variant="error" className="mb-4">
            {error}
          </Alert>
        )}

        {fetchError && (
          <Alert variant="error" className="mb-4">
            Failed to load inbox: {fetchError.message}
          </Alert>
        )}

        {fetching && rows.length === 0 && (
          <AdminLoader label="Loading signal inbox..." />
        )}

        {/* Empty state */}
        {!fetching && !fetchError && rows.length === 0 && (
          <div className="bg-card border border-border rounded-lg p-12 text-center">
            <Inbox className="w-10 h-10 text-muted-foreground/40 mx-auto mb-3" />
            <h3 className="text-lg font-semibold text-foreground mb-1">Inbox empty</h3>
            <p className="text-muted-foreground text-sm">
              Nothing awaiting review. New soft-failed posts from Root Signal will land here.
            </p>
          </div>
        )}

        {/* Grouped cards */}
        <div className="space-y-5">
          {FLAG_GROUPS.map((group) => {
            const groupRows = grouped.get(group.key) ?? [];
            if (groupRows.length === 0) return null;
            const isCollapsed = collapsed.has(group.key);
            const groupSelectedCount = groupRows.filter((r) =>
              selected.has(r.post.id)
            ).length;
            const allSelected = groupSelectedCount === groupRows.length;

            return (
              <section key={group.key}>
                <header className="flex items-center gap-3 mb-2">
                  <button
                    type="button"
                    onClick={() => toggleCollapsed(group.key)}
                    className="inline-flex items-center gap-1.5 text-sm font-semibold text-foreground hover:text-admin-accent"
                  >
                    {isCollapsed ? (
                      <ChevronRight className="w-4 h-4" />
                    ) : (
                      <ChevronDown className="w-4 h-4" />
                    )}
                    {group.label}
                    <span className="ml-1 text-muted-foreground font-normal">
                      ({groupRows.length})
                    </span>
                  </button>
                  <Checkbox
                    checked={allSelected}
                    onCheckedChange={(c) => selectGroup(group.key, !!c)}
                    aria-label={`Select all ${group.label}`}
                  />
                  <span className="text-xs text-muted-foreground">{group.summary}</span>
                </header>

                {!isCollapsed && (
                  <div className="space-y-2">
                    {groupRows.map((row) => (
                      <InboxCard
                        key={row.post.id}
                        post={row.post as InboxCardPost}
                        reviewFlags={row.reviewFlags}
                        selected={selected.has(row.post.id)}
                        onToggleSelect={() => toggleSelect(row.post.id)}
                        onApprove={() => handleApproveOne(row.post.id)}
                        onReject={() => handleRejectOne(row.post.id)}
                        onMerge={
                          row.post.duplicateOfId
                            ? () => handleMerge(row)
                            : undefined
                        }
                        busy={busyIds.has(row.post.id)}
                      />
                    ))}
                  </div>
                )}
              </section>
            );
          })}
        </div>
      </div>

      {merging && (
        <MergeDuplicateDialog
          open={!!merging}
          onOpenChange={(open) => !open && setMerging(null)}
          incoming={merging.incoming}
          canonicalId={merging.canonicalId}
          onConfirmReject={handleConfirmMerge}
          busy={busyIds.has(merging.incoming.id)}
        />
      )}
    </div>
  );
}
