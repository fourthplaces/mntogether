"use client";

import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  useDroppable,
  useDraggable,
  type DragStartEvent,
  type DragEndEvent,
} from "@dnd-kit/core";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  EditionDetailQuery,
  RowTemplatesQuery,
  PostTemplatesQuery,
  GenerateEditionMutation,
  PublishEditionMutation,
  ArchiveEditionMutation,
  ReviewEditionMutation,
  ApproveEditionMutation,
  MoveSlotMutation,
  RemovePostFromEditionMutation,
  ChangeSlotTemplateMutation,
  ReorderEditionRowsMutation,
  AddEditionRowMutation,
  DeleteEditionRowMutation,
  AddWidgetMutation,
  UpdateWidgetMutation,
  RemoveWidgetMutation,
  AddSectionMutation,
  UpdateSectionMutation,
  DeleteSectionMutation,
  AssignRowToSectionMutation,
} from "@/lib/graphql/editions";
import type {
  EditionDetailQuery as EditionDetailQueryType,
  RowTemplatesQuery as RowTemplatesQueryType,
  PostTemplatesQuery as PostTemplatesQueryType,
} from "@/gql/graphql";

// ─── Type aliases from generated GraphQL types ───────────────────────────────

type Edition = NonNullable<EditionDetailQueryType["edition"]>;
type EditionRow = Edition["rows"][number];
type EditionSlot = EditionRow["slots"][number];
type EditionWidget = EditionRow["widgets"][number];
type EditionSection = Edition["sections"][number];
type TemplateSlotDef = EditionRow["rowTemplate"]["slots"][number];
type RowTemplate = RowTemplatesQueryType["rowTemplates"][number];
type PostTemplate = PostTemplatesQueryType["postTemplates"][number];

const WEIGHT_SPAN: Record<string, number> = { heavy: 2, medium: 1, light: 1 };

// ─── Page export ─────────────────────────────────────────────────────────────

export default function EditionDetailPage() {
  const [activeTab, setActiveTab] = useState<"layout" | "posts">("layout");
  const params = useParams();
  const id = params.id as string;

  // Shared edition query — used by both tabs
  const [{ data, fetching, error }, refetchEdition] = useQuery({
    query: EditionDetailQuery,
    variables: { id },
  });

  // Auto-review: opening a draft edition transitions it to in_review
  const [, reviewEdition] = useMutation(ReviewEditionMutation);
  const mutCtx = useMemo(
    () => ({ additionalTypenames: ["Edition", "EditionRow", "EditionSlot"] }),
    []
  );
  const hasAutoReviewed = useRef(false);
  const edition = data?.edition;
  useEffect(() => {
    if (edition && edition.status === "draft" && !hasAutoReviewed.current) {
      hasAutoReviewed.current = true;
      reviewEdition({ id }, mutCtx).then((res) => {
        if (!res.error) refetchEdition({ requestPolicy: "network-only" });
      });
    }
  }, [edition?.status, id, mutCtx, reviewEdition, refetchEdition]);

  if (fetching && !edition) {
    return <AdminLoader label="Loading edition..." />;
  }

  if (error || !edition) {
    return (
      <div className="min-h-screen bg-[#FDFCFA] p-6">
        <div className="max-w-6xl mx-auto">
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">
            {error?.message || "Edition not found"}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#FDFCFA] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Tab bar */}
        <div className="flex gap-1 mb-6 border-b border-stone-200">
          <button
            onClick={() => setActiveTab("layout")}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === "layout"
                ? "border-amber-600 text-amber-700"
                : "border-transparent text-stone-500 hover:text-stone-700"
            }`}
          >
            Layout
          </button>
          <button
            onClick={() => setActiveTab("posts")}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === "posts"
                ? "border-amber-600 text-amber-700"
                : "border-transparent text-stone-500 hover:text-stone-700"
            }`}
          >
            Posts
          </button>
        </div>

        {activeTab === "layout" ? (
          <BroadsheetEditor
            edition={edition}
            refetchEdition={refetchEdition}
          />
        ) : (
          <EditionPostsView edition={edition} />
        )}
      </div>
    </div>
  );
}

// ─── Main editor component ───────────────────────────────────────────────────

function BroadsheetEditor({
  edition,
  refetchEdition,
}: {
  edition: Edition;
  refetchEdition: (opts?: any) => void;
}) {
  const router = useRouter();
  const id = edition.id;
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [activeSlotId, setActiveSlotId] = useState<string | null>(null);

  // Queries (templates only — edition comes from parent)
  const [{ data: rowTemplatesData }] = useQuery({ query: RowTemplatesQuery });
  const [{ data: postTemplatesData }] = useQuery({ query: PostTemplatesQuery });

  // Mutations
  const mutCtx = useMemo(
    () => ({ additionalTypenames: ["Edition", "EditionRow", "EditionSlot", "EditionSection"] }),
    []
  );
  const [, generateEdition] = useMutation(GenerateEditionMutation);
  const [, publishEdition] = useMutation(PublishEditionMutation);
  const [, archiveEdition] = useMutation(ArchiveEditionMutation);
  const [, reviewEdition] = useMutation(ReviewEditionMutation);
  const [, approveEdition] = useMutation(ApproveEditionMutation);
  const [, moveSlot] = useMutation(MoveSlotMutation);
  const [, removePost] = useMutation(RemovePostFromEditionMutation);
  const [, changeSlotTemplate] = useMutation(ChangeSlotTemplateMutation);
  const [, reorderRows] = useMutation(ReorderEditionRowsMutation);
  const [, addRow] = useMutation(AddEditionRowMutation);
  const [, deleteRowMut] = useMutation(DeleteEditionRowMutation);
  const [, addWidgetMut] = useMutation(AddWidgetMutation);
  const [, updateWidgetMut] = useMutation(UpdateWidgetMutation);
  const [, removeWidgetMut] = useMutation(RemoveWidgetMutation);
  const [, addSectionMut] = useMutation(AddSectionMutation);
  const [, updateSectionMut] = useMutation(UpdateSectionMutation);
  const [, deleteSectionMut] = useMutation(DeleteSectionMutation);
  const [, assignRowToSectionMut] = useMutation(AssignRowToSectionMutation);

  const rowTemplates = rowTemplatesData?.rowTemplates ?? [];
  const postTemplates = postTemplatesData?.postTemplates ?? [];
  const sections = useMemo(
    () => edition ? [...edition.sections].sort((a, b) => a.sortOrder - b.sortOrder) : [],
    [edition]
  );

  // DnD
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } })
  );

  const handleDragStart = useCallback((event: DragStartEvent) => {
    setActiveSlotId(event.active.id as string);
  }, []);

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      setActiveSlotId(null);
      const { active, over } = event;
      if (!over || !edition) return;

      const slotId = active.id as string;
      const overId = over.id as string;

      if (overId === "remove-zone") {
        await removePost({ slotId }, mutCtx);
        refetchEdition({ requestPolicy: "network-only" });
        return;
      }

      // Parse droppable: "drop-{rowId}-{slotIndex}" (rowId is a UUID with hyphens)
      const match = overId.match(/^drop-(.+)-(\d+)$/);
      if (match) {
        const targetRowId = match[1];
        const slotIndex = parseInt(match[2], 10);
        await moveSlot({ slotId, targetRowId, slotIndex }, mutCtx);
        refetchEdition({ requestPolicy: "network-only" });
      }
    },
    [edition, moveSlot, removePost, mutCtx, refetchEdition]
  );

  // Row management
  const sortedRows = useMemo(
    () =>
      edition
        ? [...edition.rows].sort((a, b) => a.sortOrder - b.sortOrder)
        : [],
    [edition]
  );

  const handleMoveRow = useCallback(
    async (rowId: string, direction: "up" | "down") => {
      const idx = sortedRows.findIndex((r) => r.id === rowId);
      if (idx < 0) return;
      const swapIdx = direction === "up" ? idx - 1 : idx + 1;
      if (swapIdx < 0 || swapIdx >= sortedRows.length) return;
      const newOrder = sortedRows.map((r) => r.id);
      [newOrder[idx], newOrder[swapIdx]] = [newOrder[swapIdx], newOrder[idx]];
      await reorderRows({ editionId: edition!.id, rowIds: newOrder }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [sortedRows, edition, reorderRows, mutCtx, refetchEdition]
  );

  const handleDeleteRow = useCallback(
    async (rowId: string) => {
      await deleteRowMut({ rowId }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [deleteRowMut, mutCtx, refetchEdition]
  );

  const handleAddRow = useCallback(
    async (rowTemplateSlug: string) => {
      const nextOrder =
        sortedRows.length > 0
          ? Math.max(...sortedRows.map((r) => r.sortOrder)) + 1
          : 0;
      await addRow(
        { editionId: edition!.id, rowTemplateSlug, sortOrder: nextOrder },
        mutCtx
      );
      refetchEdition({ requestPolicy: "network-only" });
    },
    [edition, sortedRows, addRow, mutCtx, refetchEdition]
  );

  const handleStatusAction = useCallback(
    async (action: "generate" | "approve" | "publish" | "archive") => {
      setActionError(null);
      setActionSuccess(null);
      const fns = {
        generate: generateEdition,
        approve: approveEdition,
        publish: publishEdition,
        archive: archiveEdition,
      };
      const labels: Record<string, string> = {
        generate: "Layout regenerated",
        approve: "Edition approved",
        publish: "Edition published",
        archive: "Edition archived",
      };
      const result = await fns[action]({ id }, mutCtx);
      if (result.error) {
        setActionError(result.error.message);
      } else {
        setActionSuccess(labels[action] ?? "Done");
        refetchEdition({ requestPolicy: "network-only" });
        // Auto-dismiss success after 4s
        setTimeout(() => setActionSuccess(null), 4000);
      }
    },
    [id, mutCtx, generateEdition, approveEdition, publishEdition, archiveEdition, refetchEdition]
  );

  const handleChangeTemplate = useCallback(
    async (slotId: string, postTemplate: string) => {
      await changeSlotTemplate({ slotId, postTemplate }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [changeSlotTemplate, mutCtx, refetchEdition]
  );

  const handleRemovePost = useCallback(
    async (slotId: string) => {
      await removePost({ slotId }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [removePost, mutCtx, refetchEdition]
  );

  const handleAddWidget = useCallback(
    async (editionRowId: string, widgetType: string, config: Record<string, unknown>) => {
      const row = sortedRows.find((r) => r.id === editionRowId);
      const nextIndex = row ? Math.max(0, ...row.widgets.map((w) => w.slotIndex)) + 1 : 0;
      await addWidgetMut(
        { editionRowId, widgetType, slotIndex: nextIndex, config: JSON.stringify(config) },
        mutCtx
      );
      refetchEdition({ requestPolicy: "network-only" });
    },
    [sortedRows, addWidgetMut, mutCtx, refetchEdition]
  );

  const handleRemoveWidget = useCallback(
    async (widgetId: string) => {
      await removeWidgetMut({ id: widgetId }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [removeWidgetMut, mutCtx, refetchEdition]
  );

  // Section handlers
  const handleAddSection = useCallback(
    async (title: string) => {
      const nextOrder = sections.length > 0
        ? Math.max(...sections.map((s) => s.sortOrder)) + 1
        : 0;
      await addSectionMut(
        { editionId: edition!.id, title, sortOrder: nextOrder },
        mutCtx
      );
      refetchEdition({ requestPolicy: "network-only" });
    },
    [edition, sections, addSectionMut, mutCtx, refetchEdition]
  );

  const handleUpdateSection = useCallback(
    async (sectionId: string, title: string, subtitle?: string) => {
      await updateSectionMut({ id: sectionId, title, subtitle }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [updateSectionMut, mutCtx, refetchEdition]
  );

  const handleDeleteSection = useCallback(
    async (sectionId: string) => {
      await deleteSectionMut({ id: sectionId }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [deleteSectionMut, mutCtx, refetchEdition]
  );

  const handleAssignRowToSection = useCallback(
    async (rowId: string, sectionId: string | null) => {
      await assignRowToSectionMut({ rowId, sectionId }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [assignRowToSectionMut, mutCtx, refetchEdition]
  );

  const isEditable = edition.status === "in_review" || edition.status === "draft";

  const activeSlotData = activeSlotId
    ? sortedRows.flatMap((r) => r.slots).find((s) => s.id === activeSlotId)
    : null;

  return (
    <>
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <button
            onClick={() => router.push("/admin/workflow")}
            className="text-sm text-stone-500 hover:text-stone-700 mb-2 block"
          >
            &larr; Review Board
          </button>
          <h1 className="text-2xl font-bold text-stone-900">
            {edition.county.name} County
          </h1>
          <div className="flex items-center gap-3 mt-1 text-sm text-stone-500">
            <span>
              {formatDateRange(edition.periodStart, edition.periodEnd)}
            </span>
            <StatusBadge status={edition.status} />
          </div>
        </div>
        <div className="flex items-center gap-2 pt-8">
          <a
            href={`${process.env.NEXT_PUBLIC_WEB_APP_URL || "http://localhost:3001"}/preview/${edition.id}`}
            target="_blank"
            rel="noopener noreferrer"
            className="px-3 py-1.5 text-sm font-medium rounded-lg bg-indigo-100 text-indigo-800 hover:bg-indigo-200 transition-colors"
          >
            Preview Broadsheet ↗
          </a>
          {isEditable && (
            <button
              onClick={() => {
                const ok = window.confirm(
                  "Regenerating will replace all rows, posts, sections, and widgets " +
                  "with a fresh layout. Any manual edits to the broadsheet will be lost.\n\n" +
                  "Continue?"
                );
                if (ok) handleStatusAction("generate");
              }}
              className="px-3 py-1.5 text-sm font-medium rounded-lg bg-stone-200 text-stone-700 hover:bg-stone-300 transition-colors"
            >
              Regenerate
            </button>
          )}
          {edition.status === "in_review" && (
            <button
              onClick={() => handleStatusAction("approve")}
              className="px-3 py-1.5 text-sm font-medium rounded-lg bg-amber-100 text-amber-800 hover:bg-amber-200 transition-colors"
            >
              Approve
            </button>
          )}
          {edition.status === "approved" && (
            <button
              onClick={() => handleStatusAction("publish")}
              className="px-3 py-1.5 text-sm font-medium rounded-lg bg-green-600 text-white hover:bg-green-700 transition-colors"
            >
              Publish
            </button>
          )}
          {edition.status === "published" && (
            <button
              onClick={() => handleStatusAction("archive")}
              className="px-3 py-1.5 text-sm font-medium rounded-lg bg-stone-200 text-stone-700 hover:bg-stone-300 transition-colors"
            >
              Archive
            </button>
          )}
        </div>
      </div>

        {actionError && (
          <div className="mb-4 text-sm text-red-600 bg-red-50 border border-red-200 px-4 py-2 rounded-lg">
            {actionError}
          </div>
        )}

        {actionSuccess && (
          <div className="mb-4 text-sm text-green-700 bg-green-50 border border-green-200 px-4 py-2 rounded-lg flex items-center justify-between">
            <span>✓ {actionSuccess}</span>
            <button
              onClick={() => setActionSuccess(null)}
              className="text-green-400 hover:text-green-600 text-xs ml-4"
            >
              dismiss
            </button>
          </div>
        )}

        {/* Summary stats */}
        <div className="grid grid-cols-4 gap-4 mb-6">
          <StatCard value={sortedRows.length} label="Rows" />
          <StatCard
            value={sortedRows.reduce((sum, r) => sum + r.slots.length, 0)}
            label="Posts Placed"
          />
          <StatCard
            value={sections.length}
            label="Sections"
          />
          <StatCard
            value={new Set(sortedRows.map((r) => r.rowTemplate.slug)).size}
            label="Templates"
          />
        </div>

        {/* Broadsheet layout with DnD */}
        <DndContext
          sensors={sensors}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
        >
          {sortedRows.length === 0 ? (
            <div className="text-stone-500 text-center py-12 bg-white rounded-lg shadow-sm border border-stone-200">
              <p className="text-lg mb-2">Empty broadsheet</p>
              <p className="text-sm">
                Click &ldquo;Regenerate&rdquo; to auto-populate, or add rows
                manually.
              </p>
            </div>
          ) : (
            <SectionGroupedLayout
              rows={sortedRows}
              sections={sections}
              isEditable={isEditable}
              postTemplates={postTemplates}
              onMoveRow={handleMoveRow}
              onDeleteRow={handleDeleteRow}
              onChangeTemplate={handleChangeTemplate}
              onRemovePost={handleRemovePost}
              onViewPost={(postId) => router.push(`/admin/posts/${postId}`)}
              onAddWidget={handleAddWidget}
              onRemoveWidget={handleRemoveWidget}
              onUpdateSection={handleUpdateSection}
              onDeleteSection={handleDeleteSection}
              onAssignRowToSection={handleAssignRowToSection}
            />
          )}

          {activeSlotId && isEditable && <RemoveDropZone />}

          <DragOverlay>
            {activeSlotData ? <SlotCardOverlay slot={activeSlotData} /> : null}
          </DragOverlay>
        </DndContext>

      {isEditable && (
        <div className="mt-4 flex gap-3">
          {rowTemplates.length > 0 && (
            <AddRowButton templates={rowTemplates} onAdd={handleAddRow} />
          )}
          <AddSectionButton onAdd={handleAddSection} />
        </div>
      )}
    </>
  );
}

// ─── Edition Posts View ─────────────────────────────────────────────────────

function EditionPostsView({ edition }: { edition: Edition }) {
  const router = useRouter();

  // Extract all posts from edition slots
  const posts = useMemo(() => {
    const allPosts: Array<{
      id: string;
      title: string;
      postType: string | null | undefined;
      weight: string | null | undefined;
      status: string;
      rowTemplate: string;
      slotIndex: number;
    }> = [];

    for (const row of edition.rows) {
      for (const slot of row.slots) {
        if (slot.post) {
          allPosts.push({
            id: slot.post.id,
            title: slot.post.title,
            postType: slot.post.postType,
            weight: slot.post.weight,
            status: slot.post.status,
            rowTemplate: row.rowTemplate.displayName,
            slotIndex: slot.slotIndex,
          });
        }
      }
    }

    return allPosts.sort((a, b) => a.title.localeCompare(b.title));
  }, [edition]);

  return (
    <>
      {/* Header */}
      <div className="flex items-start justify-between mb-6">
        <div>
          <button
            onClick={() => router.push("/admin/workflow")}
            className="text-sm text-stone-500 hover:text-stone-700 mb-2 block"
          >
            &larr; Review Board
          </button>
          <h1 className="text-2xl font-bold text-stone-900">
            {edition.county.name} County &mdash; Posts
          </h1>
          <div className="flex items-center gap-3 mt-1 text-sm text-stone-500">
            <span>
              {formatDateRange(edition.periodStart, edition.periodEnd)}
            </span>
            <StatusBadge status={edition.status} />
            <span>{posts.length} post{posts.length !== 1 ? "s" : ""} placed</span>
          </div>
        </div>
      </div>

      {posts.length === 0 ? (
        <div className="text-stone-500 text-center py-12 text-sm">
          No posts placed in this edition yet.
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {posts.map((post) => (
            <div
              key={post.id}
              onClick={() => router.push(`/admin/posts/${post.id}`)}
              className="bg-white rounded-lg border border-stone-200 p-4 hover:shadow-md transition-shadow cursor-pointer"
            >
              <div className="text-sm font-medium text-stone-900 mb-2">
                {post.title}
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <PostTypeBadge type={post.postType} />
                {post.weight && <WeightBadge weight={post.weight} />}
                <span className="text-xs text-stone-400">
                  {post.rowTemplate}
                </span>
                <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                  post.status === "active"
                    ? "bg-green-100 text-green-800"
                    : post.status === "pending_approval"
                      ? "bg-yellow-100 text-yellow-800"
                      : post.status === "rejected"
                        ? "bg-red-100 text-red-800"
                        : "bg-stone-100 text-stone-600"
                }`}>
                  {post.status === "pending_approval" ? "Pending" : post.status}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

// ─── RowEditor ───────────────────────────────────────────────────────────────

function RowEditor({
  row,
  rowIndex,
  totalRows,
  isEditable,
  postTemplates,
  sections,
  onMoveRow,
  onDeleteRow,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddWidget,
  onRemoveWidget,
  onAssignRowToSection,
}: {
  row: EditionRow;
  rowIndex: number;
  totalRows: number;
  isEditable: boolean;
  postTemplates: PostTemplate[];
  sections?: EditionSection[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddWidget: (rowId: string, widgetType: string, config: Record<string, unknown>) => void;
  onRemoveWidget: (widgetId: string) => void;
  onAssignRowToSection?: (rowId: string, sectionId: string | null) => void;
}) {
  const templateSlots = useMemo(
    () => [...row.rowTemplate.slots].sort((a, b) => a.slotIndex - b.slotIndex),
    [row.rowTemplate.slots]
  );

  const slotsByIndex = useMemo(() => {
    const map = new Map<number, EditionSlot[]>();
    for (const slot of row.slots) {
      const existing = map.get(slot.slotIndex) ?? [];
      existing.push(slot);
      map.set(slot.slotIndex, existing);
    }
    return map;
  }, [row.slots]);

  const sortedWidgets = useMemo(
    () => [...row.widgets].sort((a, b) => a.slotIndex - b.slotIndex),
    [row.widgets]
  );

  return (
    <div className="bg-white rounded-lg shadow-sm border border-stone-200 overflow-hidden">
      {/* Row header */}
      <div className="px-4 py-2.5 bg-stone-50 border-b border-stone-200 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className="text-xs font-mono text-stone-400 bg-stone-200/70 rounded px-1.5 py-0.5">
            {rowIndex + 1}
          </span>
          <span className="text-sm font-semibold text-stone-700">
            {row.rowTemplate.displayName}
          </span>
          <span className="text-xs text-stone-400">{row.rowTemplate.slug}</span>
        </div>
        <div className="flex items-center gap-1">
          {isEditable && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  className="text-[11px] font-medium text-amber-600 hover:text-amber-700 px-2 py-1 rounded hover:bg-amber-50 transition-colors"
                >
                  + Widget
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-52">
                <DropdownMenuItem onClick={() => onAddWidget(row.id, "section_header", { title: "Section Title" })}>
                  <div>
                    <div className="font-medium">Section Header</div>
                    <div className="text-xs text-stone-400">Full-width divider with heading</div>
                  </div>
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => onAddWidget(row.id, "weather", {})}>
                  <div>
                    <div className="font-medium">Weather</div>
                    <div className="text-xs text-stone-400">County weather forecast card</div>
                  </div>
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => onAddWidget(row.id, "hotline_bar", { lines: [{ label: "Crisis Line", phone: "988" }] })}>
                  <div>
                    <div className="font-medium">Hotline Bar</div>
                    <div className="text-xs text-stone-400">Phone numbers and resources</div>
                  </div>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
          {isEditable && sections && sections.length > 0 && onAssignRowToSection && (
            <Select
              value={row.sectionId ?? "__none__"}
              onValueChange={(val) => onAssignRowToSection(row.id, val === "__none__" ? null : val)}
            >
              <SelectTrigger className="h-6 text-[10px] px-2 py-0 min-w-0 w-auto border-stone-200 max-w-[140px]">
                <SelectValue placeholder="No section" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__none__">No section</SelectItem>
                {sections.map((s) => (
                  <SelectItem key={s.id} value={s.id}>
                    {s.title}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
          {isEditable && (
            <>
              <button
                onClick={() => onMoveRow(row.id, "up")}
                disabled={rowIndex === 0}
                className="p-1 text-stone-400 hover:text-stone-600 disabled:opacity-30 text-sm"
                title="Move up"
              >
                &uarr;
              </button>
              <button
                onClick={() => onMoveRow(row.id, "down")}
                disabled={rowIndex === totalRows - 1}
                className="p-1 text-stone-400 hover:text-stone-600 disabled:opacity-30 text-sm"
                title="Move down"
              >
                &darr;
              </button>
              <button
                onClick={() => onDeleteRow(row.id)}
                className="p-1 text-red-400 hover:text-red-600 ml-2 text-sm"
                title="Delete row"
              >
                &times;
              </button>
            </>
          )}
        </div>
      </div>

      {/* Widgets above the slot grid */}
      {sortedWidgets.length > 0 && (
        <div className="px-4 pt-3 space-y-2">
          {sortedWidgets.map((widget) => (
            <WidgetCard
              key={widget.id}
              widget={widget}
              isEditable={isEditable}
              onRemove={onRemoveWidget}
            />
          ))}
        </div>
      )}

      {/* Slot grid */}
      <div className="p-4">
        <div className="grid grid-cols-3 gap-3">
          {templateSlots.map((tSlot) => (
            <SlotCell
              key={tSlot.slotIndex}
              rowId={row.id}
              templateSlot={tSlot}
              editionSlots={slotsByIndex.get(tSlot.slotIndex) ?? []}
              isEditable={isEditable}
              postTemplates={postTemplates}
              onChangeTemplate={onChangeTemplate}
              onRemovePost={onRemovePost}
              onViewPost={onViewPost}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

// ─── WidgetCard ─────────────────────────────────────────────────────────────

function WidgetCard({
  widget,
  isEditable,
  onRemove,
}: {
  widget: EditionWidget;
  isEditable: boolean;
  onRemove: (id: string) => void;
}) {
  const config = parseWidgetConfig(widget.config);

  return (
    <div className="flex items-center gap-3 rounded-lg border border-stone-200 bg-stone-50/50 px-3 py-2">
      <WidgetIcon type={widget.widgetType} />
      <div className="flex-1 min-w-0">
        <WidgetContent type={widget.widgetType} config={config} />
      </div>
      {isEditable && (
        <button
          onClick={() => onRemove(widget.id)}
          className="text-[11px] text-red-400 hover:text-red-600 font-medium shrink-0"
        >
          Remove
        </button>
      )}
    </div>
  );
}

function WidgetIcon({ type }: { type: string }) {
  const icons: Record<string, { bg: string; label: string }> = {
    section_header: { bg: "bg-blue-100 text-blue-700", label: "H" },
    weather: { bg: "bg-sky-100 text-sky-700", label: "W" },
    hotline_bar: { bg: "bg-rose-100 text-rose-700", label: "P" },
  };
  const icon = icons[type] ?? { bg: "bg-stone-100 text-stone-600", label: "?" };
  return (
    <div className={`w-7 h-7 rounded flex items-center justify-center text-xs font-bold shrink-0 ${icon.bg}`}>
      {icon.label}
    </div>
  );
}

function WidgetContent({ type, config }: { type: string; config: Record<string, unknown> }) {
  switch (type) {
    case "section_header":
      return (
        <div>
          <div className="text-xs font-medium text-stone-500 uppercase tracking-wide">Section Header</div>
          <div className="text-sm font-semibold text-stone-800 truncate">
            {(config.title as string) || "Untitled"}
          </div>
          {typeof config.subtitle === "string" && config.subtitle && (
            <div className="text-xs text-stone-400 truncate">{config.subtitle}</div>
          )}
        </div>
      );
    case "weather":
      return (
        <div>
          <div className="text-xs font-medium text-stone-500 uppercase tracking-wide">Weather</div>
          <div className="text-sm text-stone-700">
            {config.location_id ? `Location: ${config.location_id}` : "County default"}
          </div>
        </div>
      );
    case "hotline_bar": {
      const lines = Array.isArray(config.lines) ? config.lines : [];
      return (
        <div>
          <div className="text-xs font-medium text-stone-500 uppercase tracking-wide">Hotline Bar</div>
          <div className="text-sm text-stone-700">
            {lines.length > 0
              ? lines.map((l: Record<string, unknown>) => (l.label as string) || "Line").join(", ")
              : "No lines configured"}
          </div>
        </div>
      );
    }
    default:
      return (
        <div>
          <div className="text-xs font-medium text-stone-500 uppercase tracking-wide">{type}</div>
          <div className="text-xs text-stone-400">Unknown widget type</div>
        </div>
      );
  }
}

function parseWidgetConfig(config: string | null | undefined): Record<string, unknown> {
  if (!config) return {};
  try {
    return JSON.parse(config) as Record<string, unknown>;
  } catch {
    return {};
  }
}

// ─── SectionGroupedLayout ────────────────────────────────────────────────────

function SectionGroupedLayout({
  rows,
  sections,
  isEditable,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddWidget,
  onRemoveWidget,
  onUpdateSection,
  onDeleteSection,
  onAssignRowToSection,
}: {
  rows: EditionRow[];
  sections: EditionSection[];
  isEditable: boolean;
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddWidget: (rowId: string, widgetType: string, config: Record<string, unknown>) => void;
  onRemoveWidget: (widgetId: string) => void;
  onUpdateSection: (sectionId: string, title: string, subtitle?: string) => void;
  onDeleteSection: (sectionId: string) => void;
  onAssignRowToSection: (rowId: string, sectionId: string | null) => void;
}) {
  // Partition rows: ungrouped (no sectionId) + grouped by section
  const ungroupedRows = rows.filter((r) => !r.sectionId);
  const rowsBySection = useMemo(() => {
    const map = new Map<string, EditionRow[]>();
    for (const row of rows) {
      if (row.sectionId) {
        const bucket = map.get(row.sectionId) ?? [];
        bucket.push(row);
        map.set(row.sectionId, bucket);
      }
    }
    return map;
  }, [rows]);

  return (
    <div className="space-y-4">
      {/* Above the fold — ungrouped rows */}
      {ungroupedRows.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center gap-2 px-1">
            <span className="text-xs font-semibold text-stone-400 uppercase tracking-wider">
              Above the Fold
            </span>
            <span className="text-xs text-stone-300">
              ({ungroupedRows.length} row{ungroupedRows.length !== 1 ? "s" : ""})
            </span>
          </div>
          {ungroupedRows.map((row, idx) => (
            <RowEditor
              key={row.id}
              row={row}
              rowIndex={idx}
              totalRows={rows.length}
              isEditable={isEditable}
              postTemplates={postTemplates}
              sections={sections}
              onMoveRow={onMoveRow}
              onDeleteRow={onDeleteRow}
              onChangeTemplate={onChangeTemplate}
              onRemovePost={onRemovePost}
              onViewPost={onViewPost}
              onAddWidget={onAddWidget}
              onRemoveWidget={onRemoveWidget}
              onAssignRowToSection={onAssignRowToSection}
            />
          ))}
        </div>
      )}

      {/* Topic sections */}
      {sections.map((section) => {
        const sectionRows = rowsBySection.get(section.id) ?? [];
        return (
          <SectionBlock
            key={section.id}
            section={section}
            rows={sectionRows}
            allRows={rows}
            sections={sections}
            isEditable={isEditable}
            postTemplates={postTemplates}
            onMoveRow={onMoveRow}
            onDeleteRow={onDeleteRow}
            onChangeTemplate={onChangeTemplate}
            onRemovePost={onRemovePost}
            onViewPost={onViewPost}
            onAddWidget={onAddWidget}
            onRemoveWidget={onRemoveWidget}
            onUpdateSection={onUpdateSection}
            onDeleteSection={onDeleteSection}
            onAssignRowToSection={onAssignRowToSection}
          />
        );
      })}
    </div>
  );
}

// ─── SectionBlock ────────────────────────────────────────────────────────────

function SectionBlock({
  section,
  rows,
  allRows,
  sections,
  isEditable,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddWidget,
  onRemoveWidget,
  onUpdateSection,
  onDeleteSection,
  onAssignRowToSection,
}: {
  section: EditionSection;
  rows: EditionRow[];
  allRows: EditionRow[];
  sections: EditionSection[];
  isEditable: boolean;
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddWidget: (rowId: string, widgetType: string, config: Record<string, unknown>) => void;
  onRemoveWidget: (widgetId: string) => void;
  onUpdateSection: (sectionId: string, title: string, subtitle?: string) => void;
  onDeleteSection: (sectionId: string) => void;
  onAssignRowToSection: (rowId: string, sectionId: string | null) => void;
}) {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState(section.title);
  const [editSubtitle, setEditSubtitle] = useState(section.subtitle ?? "");

  const handleSave = () => {
    onUpdateSection(section.id, editTitle, editSubtitle || undefined);
    setIsEditing(false);
  };

  return (
    <div className="space-y-4">
      {/* Section header */}
      <div className="flex items-center gap-3 px-1 py-2 border-b-2 border-amber-300">
        <button
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="text-stone-400 hover:text-stone-600 text-sm w-5 text-center"
        >
          {isCollapsed ? "\u25B6" : "\u25BC"}
        </button>

        {isEditing ? (
          <div className="flex items-center gap-2 flex-1">
            <input
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              className="text-sm font-semibold text-stone-800 border border-stone-300 rounded px-2 py-1 flex-1 max-w-xs"
              placeholder="Section title"
              autoFocus
            />
            <input
              value={editSubtitle}
              onChange={(e) => setEditSubtitle(e.target.value)}
              className="text-xs text-stone-500 border border-stone-300 rounded px-2 py-1 flex-1 max-w-xs"
              placeholder="Subtitle (optional)"
            />
            <button
              onClick={handleSave}
              className="text-xs font-medium text-green-600 hover:text-green-700 px-2 py-1"
            >
              Save
            </button>
            <button
              onClick={() => setIsEditing(false)}
              className="text-xs font-medium text-stone-400 hover:text-stone-600 px-2 py-1"
            >
              Cancel
            </button>
          </div>
        ) : (
          <div className="flex items-center gap-2 flex-1">
            <span className="text-sm font-semibold text-stone-800">
              {section.title}
            </span>
            {section.subtitle && (
              <span className="text-xs text-stone-400">&mdash; {section.subtitle}</span>
            )}
            {section.topicSlug && (
              <span className="text-[10px] font-mono text-amber-600 bg-amber-50 px-1.5 py-0.5 rounded">
                {section.topicSlug}
              </span>
            )}
            <span className="text-xs text-stone-300">
              ({rows.length} row{rows.length !== 1 ? "s" : ""})
            </span>
          </div>
        )}

        {isEditable && !isEditing && (
          <div className="flex items-center gap-1">
            <button
              onClick={() => {
                setEditTitle(section.title);
                setEditSubtitle(section.subtitle ?? "");
                setIsEditing(true);
              }}
              className="text-[11px] font-medium text-stone-400 hover:text-stone-600 px-2 py-1"
            >
              Edit
            </button>
            <button
              onClick={() => onDeleteSection(section.id)}
              className="text-[11px] font-medium text-red-400 hover:text-red-600 px-2 py-1"
            >
              Delete
            </button>
          </div>
        )}
      </div>

      {/* Section rows (collapsible) */}
      {!isCollapsed && (
        <div className="space-y-4 pl-4 border-l-2 border-amber-100">
          {rows.length === 0 ? (
            <div className="text-xs text-stone-400 py-4 text-center">
              No rows in this section. Drag rows here or assign from row menus.
            </div>
          ) : (
            rows.map((row, idx) => (
              <RowEditor
                key={row.id}
                row={row}
                rowIndex={idx}
                totalRows={allRows.length}
                isEditable={isEditable}
                postTemplates={postTemplates}
                sections={sections}
                onMoveRow={onMoveRow}
                onDeleteRow={onDeleteRow}
                onChangeTemplate={onChangeTemplate}
                onRemovePost={onRemovePost}
                onViewPost={onViewPost}
                onAddWidget={onAddWidget}
                onRemoveWidget={onRemoveWidget}
                onAssignRowToSection={onAssignRowToSection}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}

// ─── AddSectionButton ────────────────────────────────────────────────────────

function AddSectionButton({ onAdd }: { onAdd: (title: string) => void }) {
  const [isOpen, setIsOpen] = useState(false);
  const [title, setTitle] = useState("");

  const handleSubmit = () => {
    if (title.trim()) {
      onAdd(title.trim());
      setTitle("");
      setIsOpen(false);
    }
  };

  if (!isOpen) {
    return (
      <button
        onClick={() => setIsOpen(true)}
        className="flex-1 py-3 rounded-lg border-2 border-dashed border-amber-300 text-sm font-medium text-amber-600 hover:border-amber-400 hover:text-amber-700 transition-colors"
      >
        + Add Section
      </button>
    );
  }

  return (
    <div className="flex-1 flex items-center gap-2 py-2 px-3 rounded-lg border border-amber-300 bg-amber-50/50">
      <input
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
        placeholder="Section title..."
        className="text-sm flex-1 bg-transparent border-none outline-none text-stone-800 placeholder-stone-400"
        autoFocus
      />
      <button
        onClick={handleSubmit}
        disabled={!title.trim()}
        className="text-xs font-medium text-amber-700 hover:text-amber-800 disabled:text-stone-300 px-2 py-1"
      >
        Add
      </button>
      <button
        onClick={() => { setIsOpen(false); setTitle(""); }}
        className="text-xs font-medium text-stone-400 hover:text-stone-600 px-2 py-1"
      >
        Cancel
      </button>
    </div>
  );
}

// ─── SlotCell (droppable grid cell) ──────────────────────────────────────────

function SlotCell({
  rowId,
  templateSlot,
  editionSlots,
  isEditable,
  postTemplates,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
}: {
  rowId: string;
  templateSlot: TemplateSlotDef;
  editionSlots: EditionSlot[];
  isEditable: boolean;
  postTemplates: PostTemplate[];
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
}) {
  const droppableId = `drop-${rowId}-${templateSlot.slotIndex}`;
  const { isOver, setNodeRef } = useDroppable({
    id: droppableId,
    disabled: !isEditable,
  });
  const colSpan = WEIGHT_SPAN[templateSlot.weight] ?? 1;
  const hasRoom = editionSlots.length < templateSlot.count;

  return (
    <div
      ref={setNodeRef}
      className={`space-y-2 rounded-lg p-2 transition-colors min-h-[80px] ${
        isOver
          ? "bg-amber-50 ring-2 ring-amber-300"
          : hasRoom && isEditable
            ? "bg-stone-50/50"
            : ""
      }`}
      style={{ gridColumn: `span ${colSpan}` }}
    >
      {editionSlots.map((slot) => (
        <DraggableSlotCard
          key={slot.id}
          slot={slot}
          isEditable={isEditable}
          postTemplates={postTemplates}
          onChangeTemplate={onChangeTemplate}
          onRemovePost={onRemovePost}
          onViewPost={onViewPost}
        />
      ))}
      {hasRoom && isEditable && (
        <div
          className={`rounded-lg border-2 border-dashed p-3 flex items-center justify-center ${
            isOver ? "border-amber-400 bg-amber-50/50" : "border-stone-200"
          }`}
        >
          <span className="text-xs text-stone-400">
            {isOver
              ? "Drop here"
              : `${templateSlot.weight} \u00b7 ${templateSlot.count - editionSlots.length} open`}
          </span>
        </div>
      )}
      {editionSlots.length === 0 && !isEditable && (
        <div className="rounded-lg border-2 border-dashed border-stone-200 p-4 flex items-center justify-center">
          <span className="text-xs text-stone-400">Empty</span>
        </div>
      )}
    </div>
  );
}

// ─── DraggableSlotCard ───────────────────────────────────────────────────────

function DraggableSlotCard({
  slot,
  isEditable,
  postTemplates,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
}: {
  slot: EditionSlot;
  isEditable: boolean;
  postTemplates: PostTemplate[];
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, isDragging } =
    useDraggable({ id: slot.id, disabled: !isEditable });

  const style = transform
    ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }
    : undefined;

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`rounded-lg border border-stone-200 bg-white p-3 transition-shadow ${
        isDragging ? "opacity-30 shadow-lg" : "hover:shadow-md"
      } ${isEditable ? "cursor-grab active:cursor-grabbing" : ""}`}
      {...(isEditable ? { ...attributes, ...listeners } : {})}
    >
      <div className="text-sm font-medium text-stone-900 truncate">
        {slot.post.title}
      </div>
      <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
        <PostTypeBadge type={slot.post.postType} />
        <WeightBadge weight={slot.post.weight} />
        <span className="text-[10px] text-stone-400">{slot.postTemplate}</span>
      </div>

      <div className="flex items-center gap-2 mt-2 pt-2 border-t border-stone-100">
        <button
          onClick={(e) => { e.stopPropagation(); onViewPost(slot.post.id); }}
          className="text-[11px] text-amber-600 hover:text-amber-700 font-medium"
          onPointerDown={(e) => e.stopPropagation()}
        >
          View
        </button>
        {isEditable && postTemplates.length > 0 && (
          <Select
            value={slot.postTemplate}
            onValueChange={(val) => onChangeTemplate(slot.id, val)}
          >
            <SelectTrigger
              className="h-6 text-[11px] px-2 py-0 min-w-0 w-auto border-stone-200"
              onPointerDown={(e) => e.stopPropagation()}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {postTemplates.map((pt) => (
                <SelectItem key={pt.slug} value={pt.slug}>
                  {pt.displayName}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
        {isEditable && (
          <button
            onClick={(e) => { e.stopPropagation(); onRemovePost(slot.id); }}
            className="text-[11px] text-red-500 hover:text-red-700 font-medium ml-auto"
            onPointerDown={(e) => e.stopPropagation()}
          >
            Remove
          </button>
        )}
      </div>
    </div>
  );
}

// ─── RemoveDropZone ──────────────────────────────────────────────────────────

function RemoveDropZone() {
  const { isOver, setNodeRef } = useDroppable({ id: "remove-zone" });
  return (
    <div
      ref={setNodeRef}
      className={`mt-6 rounded-lg border-2 border-dashed p-4 text-center transition-colors ${
        isOver
          ? "border-red-400 bg-red-50 text-red-600"
          : "border-stone-300 bg-stone-50 text-stone-400"
      }`}
    >
      <span className="text-sm font-medium">
        {isOver ? "Release to remove" : "Drag here to remove post"}
      </span>
    </div>
  );
}

// ─── SlotCardOverlay (drag ghost) ────────────────────────────────────────────

function SlotCardOverlay({ slot }: { slot: EditionSlot }) {
  return (
    <div className="rounded-lg border border-amber-300 bg-white shadow-xl p-3 max-w-xs">
      <div className="text-sm font-medium text-stone-900 truncate">
        {slot.post.title}
      </div>
      <div className="flex items-center gap-1.5 mt-1">
        <PostTypeBadge type={slot.post.postType} />
        <WeightBadge weight={slot.post.weight} />
      </div>
    </div>
  );
}

// ─── AddRowButton ────────────────────────────────────────────────────────────

function AddRowButton({
  templates,
  onAdd,
}: {
  templates: RowTemplate[];
  onAdd: (slug: string) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button className="w-full py-3 rounded-lg border-2 border-dashed border-stone-300 text-sm font-medium text-stone-500 hover:border-stone-400 hover:text-stone-600 transition-colors">
          + Add Row
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="center" className="w-64">
        {templates.map((t) => (
          <DropdownMenuItem key={t.slug} onClick={() => onAdd(t.slug)}>
            <div>
              <div className="font-medium">{t.displayName}</div>
              {t.description && (
                <div className="text-xs text-stone-400 mt-0.5">
                  {t.description}
                </div>
              )}
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

// ─── Shared UI helpers ───────────────────────────────────────────────────────

function StatCard({ value, label }: { value: number; label: string }) {
  return (
    <div className="bg-white rounded-lg shadow-sm border border-stone-200 px-4 py-3 text-center">
      <div className="text-2xl font-bold text-stone-900">{value}</div>
      <div className="text-xs text-stone-500 uppercase tracking-wider">
        {label}
      </div>
    </div>
  );
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    draft: "bg-yellow-100 text-yellow-800",
    in_review: "bg-amber-100 text-amber-800",
    approved: "bg-emerald-100 text-emerald-800",
    published: "bg-green-100 text-green-800",
    archived: "bg-stone-100 text-stone-600",
  };
  const labels: Record<string, string> = {
    draft: "Ready for Review",
    in_review: "In Review",
    approved: "Approved",
    published: "Published",
    archived: "Archived",
  };
  return (
    <span
      className={`px-2 py-0.5 text-xs rounded-full font-medium ${
        styles[status] || "bg-stone-100 text-stone-600"
      }`}
    >
      {labels[status] || status}
    </span>
  );
}

function PostTypeBadge({ type }: { type: string | null | undefined }) {
  if (!type) return null;
  const colors: Record<string, string> = {
    story: "bg-blue-100 text-blue-700",
    notice: "bg-amber-100 text-amber-700",
    exchange: "bg-purple-100 text-purple-700",
    event: "bg-pink-100 text-pink-700",
    spotlight: "bg-green-100 text-green-700",
    reference: "bg-stone-100 text-stone-600",
  };
  return (
    <span
      className={`px-1.5 py-0.5 text-[10px] rounded font-medium ${
        colors[type] || "bg-stone-100 text-stone-600"
      }`}
    >
      {type}
    </span>
  );
}

function WeightBadge({ weight }: { weight: string | null | undefined }) {
  if (!weight) return null;
  const colors: Record<string, string> = {
    heavy: "bg-stone-800 text-white",
    medium: "bg-stone-400 text-white",
    light: "bg-stone-200 text-stone-600",
  };
  return (
    <span
      className={`px-1.5 py-0.5 text-[10px] rounded font-medium ${
        colors[weight] || "bg-stone-100 text-stone-600"
      }`}
    >
      {weight}
    </span>
  );
}

function formatDateRange(start: string, end: string): string {
  const s = new Date(start + "T00:00:00");
  const e = new Date(end + "T00:00:00");
  const opts: Intl.DateTimeFormatOptions = { month: "short", day: "numeric" };
  if (s.getFullYear() !== e.getFullYear()) {
    return `${s.toLocaleDateString("en-US", { ...opts, year: "numeric" })} \u2013 ${e.toLocaleDateString("en-US", { ...opts, year: "numeric" })}`;
  }
  return `${s.toLocaleDateString("en-US", opts)} \u2013 ${e.toLocaleDateString("en-US", { ...opts, year: "numeric" })}`;
}
