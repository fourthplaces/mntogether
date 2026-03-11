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
  LatestEditionsQuery,
  ReviewEditionMutation,
  ApproveEditionMutation,
  BatchPublishEditionsMutation,
} from "@/lib/graphql/editions";
import { Button } from "@/components/ui/button";

// ─── Column config (left-to-right = editorial flow) ─────────────────────────

const COLUMNS = [
  { id: "draft", label: "Draft", status: "draft", color: "bg-kanban-draft-bg" },
  { id: "in_review", label: "Reviewing", status: "in_review", color: "bg-kanban-review-bg" },
  { id: "approved", label: "Approved", status: "approved", color: "bg-kanban-published-bg" },
] as const;

type ColumnId = (typeof COLUMNS)[number]["id"];

// ─── Component ─────────────────────────────────────────────────────────────

export default function WorkflowPage() {
  const [activeCard, setActiveCard] = useState<EditionCardData | null>(null);

  const mutationContext = {
    additionalTypenames: ["Edition", "EditionConnection"],
  };

  // Fetch latest edition per county (87 results, no period filter)
  const [{ data, fetching }] = useQuery({
    query: LatestEditionsQuery,
  });

  const [, reviewEdition] = useMutation(ReviewEditionMutation);
  const [, approveEdition] = useMutation(ApproveEditionMutation);
  const [{ fetching: batchPublishing }, batchPublishEditions] = useMutation(
    BatchPublishEditionsMutation
  );

  // Map editions to card data grouped by status (alphabetical within each column)
  const editionsByColumn = useMemo(() => {
    const allEditions = data?.latestEditions ?? [];
    const result: Record<ColumnId, EditionCardData[]> = {
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
        rowCount: e.rows.length,
      };

      const status = e.status as ColumnId;
      if (result[status]) {
        result[status].push(card);
      }
      // Published/archived editions are not shown on the kanban
    }

    // Sort alphabetically by county name
    for (const col of Object.values(result)) {
      col.sort((a, b) => a.countyName.localeCompare(b.countyName));
    }

    return result;
  }, [data]);

  // Progress stats
  const totalCount = data?.latestEditions?.length ?? 0;
  const approvedCount = editionsByColumn.approved?.length ?? 0;
  const kanbanCount =
    (editionsByColumn.draft?.length ?? 0) +
    (editionsByColumn.in_review?.length ?? 0) +
    approvedCount;

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

      // Perform status transition (forward-only)
      if (sourceColumn === "draft" && targetColumn === "in_review") {
        await reviewEdition({ id: editionId }, mutationContext);
      } else if (sourceColumn === "in_review" && targetColumn === "approved") {
        await approveEdition({ id: editionId }, mutationContext);
      }
    },
    [findColumnForEdition, reviewEdition, approveEdition, mutationContext]
  );

  const handlePublishAll = useCallback(async () => {
    const approvedIds = editionsByColumn.approved.map((e) => e.id);
    if (approvedIds.length === 0) return;
    await batchPublishEditions({ ids: approvedIds }, mutationContext);
  }, [editionsByColumn, batchPublishEditions, mutationContext]);

  // Workflow guidance message
  const guidanceMessage = useMemo(() => {
    if (kanbanCount === 0 && totalCount > 0) {
      return "All counties are published and up to date.";
    }
    if (kanbanCount === 0) {
      return null; // No editions at all — empty state handles this
    }
    if (approvedCount === kanbanCount) {
      return "All editions reviewed — ready to publish!";
    }
    const reviewed = approvedCount;
    if (reviewed > 0) {
      return `${reviewed} of ${kanbanCount} editions reviewed`;
    }
    return null;
  }, [kanbanCount, approvedCount, totalCount]);

  if (fetching && !data) {
    return <AdminLoader label="Loading review board..." />;
  }

  return (
    <div className="p-6 h-full flex flex-col overflow-hidden">
      {/* Header */}
      <div className="flex items-center justify-between mb-4 shrink-0">
        <div>
          <h1 className="text-2xl font-semibold text-text-primary">
            Review Board
            <span className="ml-2 px-2 py-0.5 text-xs font-medium rounded-full bg-amber-100 text-amber-700 align-middle">
              Beta
            </span>
          </h1>
          {guidanceMessage && (
            <p className="text-sm text-text-secondary mt-0.5">
              {guidanceMessage}
            </p>
          )}
        </div>
        {approvedCount > 0 && (
          <Button
            variant="success"
            onClick={handlePublishAll}
            loading={batchPublishing}
          >
            Publish {approvedCount} Approved
          </Button>
        )}
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
        <div className="text-center py-16 text-text-faint shrink-0">
          <p className="text-sm">
            No editions found. Generate this week&rsquo;s editions from the
            Dashboard.
          </p>
        </div>
      )}
    </div>
  );
}
