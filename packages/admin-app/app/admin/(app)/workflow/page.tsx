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
import { WorkflowColumn } from "@/components/admin/WorkflowColumn";
import { WorkflowCardOverlay } from "@/components/admin/WorkflowCard";
import {
  PostsListQuery,
  ApprovePostMutation,
  ReactivatePostMutation,
} from "@/lib/graphql/posts";

// Column definitions with their target statuses
const COLUMNS = [
  { id: "draft", label: "Drafts", status: "draft", color: "bg-kanban-draft-bg" },
  { id: "pending_approval", label: "In Review", status: "pending_approval", color: "bg-kanban-review-bg" },
  { id: "active", label: "Published", status: "active", color: "bg-kanban-published-bg" },
] as const;

type ColumnId = (typeof COLUMNS)[number]["id"];

interface PostItem {
  id: string;
  title: string;
  status: string;
  postType?: string | null;
  urgency?: string | null;
  createdAt: string;
}

export default function WorkflowPage() {
  const [activeCard, setActiveCard] = useState<PostItem | null>(null);
  const [typeFilter, setTypeFilter] = useState("");

  const mutationContext = {
    additionalTypenames: ["Post", "PostConnection", "PostStats"],
  };

  // Fetch posts for each column
  const [{ data: draftData, fetching: f1 }] = useQuery({
    query: PostsListQuery,
    variables: { status: "draft", limit: 50, postType: typeFilter || undefined },
  });
  const [{ data: reviewData, fetching: f2 }] = useQuery({
    query: PostsListQuery,
    variables: { status: "pending_approval", limit: 50, postType: typeFilter || undefined },
  });
  const [{ data: activeData, fetching: f3 }] = useQuery({
    query: PostsListQuery,
    variables: { status: "active", limit: 50, postType: typeFilter || undefined },
  });

  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, reactivatePost] = useMutation(ReactivatePostMutation);

  const isLoading = f1 && f2 && f3;

  // Build post lists per column
  const postsByColumn = useMemo(() => {
    const drafts: PostItem[] = (draftData?.posts?.posts ?? []).map((p) => ({
      id: p.id,
      title: p.title,
      status: "draft",
      postType: p.postType,
      urgency: p.urgency,
      createdAt: p.createdAt,
    }));
    const reviews: PostItem[] = (reviewData?.posts?.posts ?? []).map((p) => ({
      id: p.id,
      title: p.title,
      status: "pending_approval",
      postType: p.postType,
      urgency: p.urgency,
      createdAt: p.createdAt,
    }));
    const published: PostItem[] = (activeData?.posts?.posts ?? []).map((p) => ({
      id: p.id,
      title: p.title,
      status: "active",
      postType: p.postType,
      urgency: p.urgency,
      createdAt: p.createdAt,
    }));

    return {
      draft: drafts,
      pending_approval: reviews,
      active: published,
    } as Record<ColumnId, PostItem[]>;
  }, [draftData, reviewData, activeData]);

  // Find which column a post belongs to
  const findColumnForPost = useCallback(
    (postId: string): ColumnId | null => {
      for (const col of COLUMNS) {
        if (postsByColumn[col.id].some((p) => p.id === postId)) {
          return col.id;
        }
      }
      return null;
    },
    [postsByColumn]
  );

  // DnD sensors — slight activation distance to avoid accidental drags
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragStart = useCallback(
    (event: DragStartEvent) => {
      const id = event.active.id as string;
      for (const col of COLUMNS) {
        const post = postsByColumn[col.id].find((p) => p.id === id);
        if (post) {
          setActiveCard(post);
          return;
        }
      }
    },
    [postsByColumn]
  );

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      setActiveCard(null);

      const { active, over } = event;
      if (!over) return;

      const postId = active.id as string;
      const sourceColumn = findColumnForPost(postId);

      // The drop target could be a column ID or a card ID (within a column)
      let targetColumn: ColumnId | null = null;
      if (COLUMNS.some((c) => c.id === over.id)) {
        targetColumn = over.id as ColumnId;
      } else {
        // Dropped on a card — find which column it belongs to
        targetColumn = findColumnForPost(over.id as string);
      }

      if (!targetColumn || targetColumn === sourceColumn) return;

      // Perform status transition via Restate
      if (targetColumn === "active") {
        // Moving to Published → approve
        await approvePost({ id: postId }, mutationContext);
      } else if (targetColumn === "pending_approval") {
        // Moving to In Review → reactivate (from draft or other)
        await reactivatePost({ id: postId }, mutationContext);
      }
      // Moving to Drafts would need a "set to draft" mutation — not yet wired.
      // For now, only Published and In Review transitions are supported.
    },
    [findColumnForPost, approvePost, reactivatePost, mutationContext]
  );

  if (isLoading) {
    return <AdminLoader label="Loading workflow..." />;
  }

  return (
    <div className="p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-semibold text-text-primary">Workflow</h1>

        <select
          value={typeFilter}
          onChange={(e) => setTypeFilter(e.target.value)}
          className="px-3 py-1.5 text-sm border border-border rounded-lg bg-surface-raised focus:outline-none focus:ring-2 focus:ring-focus-ring"
        >
          <option value="">All types</option>
          <option value="story">Stories</option>
          <option value="notice">Notices</option>
          <option value="exchange">Exchanges</option>
          <option value="event">Events</option>
          <option value="spotlight">Spotlights</option>
          <option value="reference">References</option>
        </select>
      </div>

      {/* Kanban board */}
      <DndContext
        sensors={sensors}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {COLUMNS.map((col) => (
            <WorkflowColumn
              key={col.id}
              id={col.id}
              label={col.label}
              count={postsByColumn[col.id].length}
              colorClass={col.color}
              posts={postsByColumn[col.id]}
            />
          ))}
        </div>

        {/* Drag overlay — renders the ghost card outside the normal flow */}
        <DragOverlay>
          {activeCard ? (
            <WorkflowCardOverlay
              id={activeCard.id}
              title={activeCard.title}
              postType={activeCard.postType}
              urgency={activeCard.urgency}
              createdAt={activeCard.createdAt}
            />
          ) : null}
        </DragOverlay>
      </DndContext>
    </div>
  );
}
