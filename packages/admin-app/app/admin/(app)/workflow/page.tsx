"use client";

import { useState, useMemo, useCallback } from "react";
import { useQuery, useMutation } from "urql";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
} from "@dnd-kit/core";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { EditionKanbanColumn } from "@/components/admin/EditionKanbanColumn";
import {
  EditionKanbanCardOverlay,
  type EditionCardData,
} from "@/components/admin/EditionKanbanCard";
import {
  EditionsListQuery,
  ReviewEditionMutation,
  ApproveEditionMutation,
  PublishEditionMutation,
  BatchPublishEditionsMutation,
} from "@/lib/graphql/editions";

// ─── Week helpers ──────────────────────────────────────────────────────────

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

function formatWeekLabel(start: string): string {
  const d = new Date(start + "T00:00:00");
  return `Week of ${d.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" })}`;
}

// ─── Column config ─────────────────────────────────────────────────────────

const COLUMNS = [
  { id: "published", label: "Live", status: "published", color: "bg-green-50" },
  { id: "draft", label: "Ready for Review", status: "draft", color: "bg-kanban-draft-bg" },
  { id: "in_review", label: "In Review", status: "in_review", color: "bg-kanban-review-bg" },
  { id: "approved", label: "Approved", status: "approved", color: "bg-kanban-published-bg" },
] as const;

type ColumnId = (typeof COLUMNS)[number]["id"];

// ─── Component ─────────────────────────────────────────────────────────────

export default function WorkflowPage() {
  const [weekOffset, setWeekOffset] = useState(0);
  const [activeCard, setActiveCard] = useState<EditionCardData | null>(null);

  // Compute current week bounds
  const { periodStart, periodEnd, weekLabel } = useMemo(() => {
    const now = new Date();
    now.setDate(now.getDate() + weekOffset * 7);
    const bounds = getWeekBounds(now);
    return {
      periodStart: bounds.start,
      periodEnd: bounds.end,
      weekLabel: formatWeekLabel(bounds.start),
    };
  }, [weekOffset]);

  const mutationContext = {
    additionalTypenames: ["Edition", "EditionConnection"],
  };

  // Fetch all editions for this period (up to 100 covers all 87 counties)
  const [{ data, fetching }] = useQuery({
    query: EditionsListQuery,
    variables: { periodStart, periodEnd, limit: 100 },
  });

  const [, reviewEdition] = useMutation(ReviewEditionMutation);
  const [, approveEdition] = useMutation(ApproveEditionMutation);
  const [, publishEdition] = useMutation(PublishEditionMutation);
  const [{ fetching: batchPublishing }, batchPublishEditions] = useMutation(
    BatchPublishEditionsMutation
  );

  // Map editions to card data grouped by status
  const editionsByColumn = useMemo(() => {
    const allEditions = data?.editions?.editions ?? [];
    const result: Record<ColumnId, EditionCardData[]> = {
      published: [],
      draft: [],
      in_review: [],
      approved: [],
    };

    for (const e of allEditions) {
      const card: EditionCardData = {
        id: e.id,
        countyName: e.county.name,
        periodStart: e.periodStart,
        periodEnd: e.periodEnd,
        status: e.status,
        filledSlots: e.rows.length, // row count as proxy for now
        totalSlots: 0, // not available from list query
      };

      const status = e.status as ColumnId;
      if (result[status]) {
        result[status].push(card);
      }
    }

    // Sort each column alphabetically by county name
    for (const col of Object.values(result)) {
      col.sort((a, b) => a.countyName.localeCompare(b.countyName));
    }

    return result;
  }, [data]);

  // Find which column an edition belongs to
  const findColumnForEdition = useCallback(
    (editionId: string): ColumnId | null => {
      for (const col of COLUMNS) {
        if (editionsByColumn[col.id].some((e) => e.id === editionId)) {
          return col.id;
        }
      }
      return null;
    },
    [editionsByColumn]
  );

  // DnD sensors
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragStart = useCallback(
    (event: DragStartEvent) => {
      const id = event.active.id as string;
      for (const col of COLUMNS) {
        const edition = editionsByColumn[col.id].find((e) => e.id === id);
        if (edition) {
          setActiveCard(edition);
          return;
        }
      }
    },
    [editionsByColumn]
  );

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      setActiveCard(null);

      const { active, over } = event;
      if (!over) return;

      const editionId = active.id as string;
      const sourceColumn = findColumnForEdition(editionId);

      // Determine target column
      let targetColumn: ColumnId | null = null;
      if (COLUMNS.some((c) => c.id === over.id)) {
        targetColumn = over.id as ColumnId;
      } else {
        targetColumn = findColumnForEdition(over.id as string);
      }

      if (!targetColumn || targetColumn === sourceColumn) return;

      // Perform status transition
      if (sourceColumn === "draft" && targetColumn === "in_review") {
        await reviewEdition({ id: editionId }, mutationContext);
      } else if (sourceColumn === "in_review" && targetColumn === "approved") {
        await approveEdition({ id: editionId }, mutationContext);
      } else if (sourceColumn === "approved" && targetColumn === "published") {
        await publishEdition({ id: editionId }, mutationContext);
      }
      // Other transitions (e.g., backwards) are not supported
    },
    [findColumnForEdition, reviewEdition, approveEdition, publishEdition, mutationContext]
  );

  const handleApproveAll = useCallback(async () => {
    const approvedIds = editionsByColumn.approved.map((e) => e.id);
    if (approvedIds.length === 0) return;
    await batchPublishEditions({ ids: approvedIds }, mutationContext);
  }, [editionsByColumn, batchPublishEditions, mutationContext]);

  if (fetching && !data) {
    return <AdminLoader label="Loading review board..." />;
  }

  const totalCount = data?.editions?.totalCount ?? 0;

  return (
    <div className="p-6 h-full flex flex-col overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between mb-4 shrink-0">
        <div>
          <h1 className="text-2xl font-semibold text-text-primary">
            Review Board
          </h1>
          <p className="text-sm text-text-secondary mt-0.5">
            {weekLabel} &middot; {totalCount} edition{totalCount !== 1 ? "s" : ""}
          </p>
        </div>

        <div className="flex items-center gap-3">
          {/* Week navigation */}
          <div className="flex items-center gap-1">
            <button
              onClick={() => setWeekOffset((w) => w - 1)}
              className="p-1.5 rounded-lg text-text-muted hover:bg-surface-muted transition-colors"
              title="Previous week"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
            </button>
            <button
              onClick={() => setWeekOffset(0)}
              className="px-2.5 py-1 rounded-lg text-xs font-medium text-text-secondary hover:bg-surface-muted transition-colors"
            >
              This Week
            </button>
            <button
              onClick={() => setWeekOffset((w) => w + 1)}
              className="p-1.5 rounded-lg text-text-muted hover:bg-surface-muted transition-colors"
              title="Next week"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
              </svg>
            </button>
          </div>
        </div>
      </div>

      {/* Kanban board — horizontal scroll, fixed-width columns (Trello-style) */}
      <DndContext
        sensors={sensors}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div className="flex gap-4 overflow-x-auto flex-1 min-h-0 -mx-6 px-6 pb-2">
          {COLUMNS.map((col) => (
            <EditionKanbanColumn
              key={col.id}
              id={col.id}
              label={col.label}
              count={editionsByColumn[col.id].length}
              colorClass={col.color}
              editions={editionsByColumn[col.id]}
              action={
                col.id === "approved" && editionsByColumn.approved.length > 0 ? (
                  <button
                    onClick={handleApproveAll}
                    disabled={batchPublishing}
                    className="text-xs font-medium px-2.5 py-1 rounded-lg bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
                  >
                    {batchPublishing ? "Publishing..." : "Publish All"}
                  </button>
                ) : undefined
              }
            />
          ))}
        </div>

        {/* Drag overlay */}
        <DragOverlay>
          {activeCard ? <EditionKanbanCardOverlay {...activeCard} /> : null}
        </DragOverlay>
      </DndContext>

      {/* Empty state */}
      {totalCount === 0 && !fetching && (
        <div className="text-center py-16 text-text-faint">
          <div className="text-4xl mb-3">📋</div>
          <p className="text-sm">
            No editions for this week. Use &ldquo;Batch Generate&rdquo; on the
            Editions page to create broadsheets.
          </p>
        </div>
      )}
    </div>
  );
}
