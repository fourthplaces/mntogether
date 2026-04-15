"use client";

import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import { useQuery, useMutation } from "urql";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  useDroppable,
  useDraggable,
  closestCenter,
  type DragStartEvent,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  ArrowLeft,
  ChevronUp,
  ChevronDown,
  X,
  Plus,
  GripVertical,
  ExternalLink,
  Lock,
  ChevronRight,
  ListStart,
  LayoutDashboard,
  Puzzle,
} from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogClose,
  DialogHeader,
  DialogFooter,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
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
  UpdateEditionRowMutation,
  DeleteEditionRowMutation,
  AddWidgetToEditionMutation,
  AddSectionMutation,
  UpdateSectionMutation,
  DeleteSectionMutation,
  AssignRowToSectionMutation,
  ReorderSectionsMutation,
} from "@/lib/graphql/editions";
import { EditionWidgetsQuery } from "@/lib/graphql/widgets";
import { EditionPostsQuery } from "@/lib/graphql/posts";
import type {
  EditionDetailQuery as EditionDetailQueryType,
  RowTemplatesQuery as RowTemplatesQueryType,
  PostTemplatesQuery as PostTemplatesQueryType,
} from "@/gql/graphql";

// ─── Type aliases from generated GraphQL types ───────────────────────────────

type Edition = NonNullable<EditionDetailQueryType["edition"]>;
type EditionRow = Edition["rows"][number];
type EditionSlot = EditionRow["slots"][number];
type EditionSection = Edition["sections"][number];
type TemplateSlotDef = EditionRow["rowTemplate"]["slots"][number];
type RowTemplate = RowTemplatesQueryType["rowTemplates"][number];
type PostTemplate = PostTemplatesQueryType["postTemplates"][number];

const WEIGHT_SPAN: Record<string, number> = { heavy: 2, medium: 1, light: 1 };

const EDITION_STATUS_LABELS: Record<string, string> = {
  draft: "Draft",
  in_review: "Reviewing",
  approved: "Approved",
  published: "Published",
  archived: "Archived",
};

// Valid status transitions (must match backend state machine in edition_ops.rs)
// draft → in_review → approved → published; archived from anywhere
const VALID_TRANSITIONS: Record<string, string[]> = {
  draft: ["in_review", "published", "archived"],
  in_review: ["approved", "archived"],
  approved: ["published", "archived"],
  published: ["archived"],
  archived: [],
};

// ─── Reusable confirmation dialog ────────────────────────────────────────────

function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel = "Confirm",
  onConfirm,
  variant = "destructive",
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel?: string;
  onConfirm: () => void;
  variant?: "destructive" | "default";
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose render={<Button variant="outline" />}>Cancel</DialogClose>
          <Button
            variant={variant === "destructive" ? "destructive" : "admin"}
            onClick={() => {
              onConfirm();
              onOpenChange(false);
            }}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ─── Layout variant SVG illustrations ────────────────────────────────────────

function LayoutVariantIcon({ variant, className }: { variant: string; className?: string }) {
  const size = className ?? "w-full h-full";
  const fill = "fill-muted-foreground/20 stroke-muted-foreground/60";
  const strokeW = 1.5;

  switch (variant) {
    case "full":
      return (
        <svg viewBox="0 0 80 48" className={size}>
          <rect x="2" y="2" width="76" height="44" rx="3" className={fill} strokeWidth={strokeW} />
        </svg>
      );
    case "lead-stack":
      return (
        <svg viewBox="0 0 80 48" className={size}>
          <rect x="2" y="2" width="50" height="44" rx="3" className={fill} strokeWidth={strokeW} />
          <rect x="56" y="2" width="22" height="12" rx="2" className={fill} strokeWidth={strokeW} />
          <rect x="56" y="18" width="22" height="12" rx="2" className={fill} strokeWidth={strokeW} />
          <rect x="56" y="34" width="22" height="12" rx="2" className={fill} strokeWidth={strokeW} />
        </svg>
      );
    case "trio":
      return (
        <svg viewBox="0 0 80 48" className={size}>
          <rect x="2" y="2" width="23" height="44" rx="3" className={fill} strokeWidth={strokeW} />
          <rect x="29" y="2" width="23" height="44" rx="3" className={fill} strokeWidth={strokeW} />
          <rect x="56" y="2" width="22" height="44" rx="3" className={fill} strokeWidth={strokeW} />
        </svg>
      );
    case "lead":
      return (
        <svg viewBox="0 0 80 48" className={size}>
          <rect x="2" y="2" width="50" height="44" rx="3" className={fill} strokeWidth={strokeW} />
          <rect x="56" y="2" width="22" height="44" rx="3" className={fill} strokeWidth={strokeW} />
        </svg>
      );
    case "pair":
      return (
        <svg viewBox="0 0 80 48" className={size}>
          <rect x="2" y="2" width="36" height="44" rx="3" className={fill} strokeWidth={strokeW} />
          <rect x="42" y="2" width="36" height="44" rx="3" className={fill} strokeWidth={strokeW} />
        </svg>
      );
    default:
      return (
        <svg viewBox="0 0 80 48" className={size}>
          <rect x="2" y="2" width="76" height="44" rx="3" className={fill} strokeWidth={strokeW} />
        </svg>
      );
  }
}

// ─── Row template picker dialog ─────────────────────────────────────────────

function RowTemplatePickerDialog({
  open,
  onOpenChange,
  templates,
  currentSlug,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  templates: RowTemplate[];
  currentSlug: string | null;
  onSelect: (slug: string) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Choose row template</DialogTitle>
          <DialogDescription>
            Select a layout for this row. Posts in existing slots will be reassigned where possible.
          </DialogDescription>
        </DialogHeader>
        <div className="grid grid-cols-3 gap-3 py-2 overflow-y-auto">
          {templates.map((t) => (
            <button
              key={t.slug}
              onClick={() => onSelect(t.slug)}
              className={`flex flex-col items-center gap-2 rounded-lg border-2 p-3 text-center transition-colors hover:bg-muted/50 ${
                t.slug === currentSlug
                  ? "border-amber-400 bg-amber-50/50"
                  : "border-border hover:border-muted-foreground/30"
              }`}
            >
              <div className="w-full h-12">
                <LayoutVariantIcon variant={t.layoutVariant} />
              </div>
              <span className="text-xs font-semibold text-foreground leading-tight">
                {t.displayName}
              </span>
              {t.description && (
                <span className="text-[10px] text-muted-foreground leading-tight">
                  {t.description}
                </span>
              )}
            </button>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}


function PostTemplatePickerDialog({
  open,
  onOpenChange,
  templates,
  currentSlug,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  templates: PostTemplate[];
  currentSlug: string | null | undefined;
  onSelect: (slug: string) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Choose post template</DialogTitle>
          <DialogDescription>
            Select how this post should be displayed in the layout.
          </DialogDescription>
        </DialogHeader>
        <div className="grid grid-cols-2 gap-3 py-2">
          {templates.map((pt) => (
            <button
              key={pt.slug}
              onClick={() => onSelect(pt.slug)}
              className={`flex flex-col items-start gap-1 rounded-lg border-2 p-3 text-left transition-colors hover:bg-muted/50 ${
                pt.slug === currentSlug
                  ? "border-amber-400 bg-amber-50/50"
                  : "border-border hover:border-muted-foreground/30"
              }`}
            >
              <span className="text-sm font-semibold text-foreground">
                {pt.displayName}
              </span>
              {pt.description && (
                <span className="text-[11px] text-muted-foreground leading-tight">
                  {pt.description}
                </span>
              )}
            </button>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function editionStatusVariant(status: string): "warning" | "success" | "secondary" {
  switch (status) {
    case "draft":
    case "in_review":
      return "warning";
    case "approved":
    case "published":
      return "success";
    default:
      return "secondary";
  }
}

// ─── Page export ─────────────────────────────────────────────────────────────

export default function EditionDetailPage() {
  const [activeTab, setActiveTab] = useState<string>("layout");
  const params = useParams();
  const router = useRouter();
  const id = params.id as string;

  // Shared edition query
  const [{ data, fetching, error }, refetchEdition] = useQuery({
    query: EditionDetailQuery,
    variables: { id },
  });

  // Mutations lifted to parent for shared header
  const mutCtx = useMemo(
    () => ({ additionalTypenames: ["Edition", "EditionRow", "EditionSlot", "EditionSection"] }),
    []
  );
  const [, reviewEdition] = useMutation(ReviewEditionMutation);
  const [, approveEdition] = useMutation(ApproveEditionMutation);
  const [, publishEdition] = useMutation(PublishEditionMutation);
  const [, archiveEdition] = useMutation(ArchiveEditionMutation);

  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [statusChanging, setStatusChanging] = useState(false);

  // Auto-review: opening a draft edition transitions it to in_review
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

  const handleStatusChange = useCallback(
    async (newStatus: string) => {
      if (!edition || newStatus === edition.status) return;
      setActionError(null);
      setActionSuccess(null);
      setStatusChanging(true);

      const fns: Record<string, { fn: (vars: { id: string }, ctx?: any) => Promise<any>; label: string }> = {
        in_review: { fn: reviewEdition, label: "Edition moved to review" },
        approved: { fn: approveEdition, label: "Edition approved" },
        published: { fn: publishEdition, label: "Edition published" },
        archived: { fn: archiveEdition, label: "Edition archived" },
      };

      const action = fns[newStatus];
      if (!action) { setStatusChanging(false); return; }

      const result = await action.fn({ id }, mutCtx);
      setStatusChanging(false);
      if (result.error) {
        setActionError(result.error.message);
      } else {
        setActionSuccess(action.label);
        refetchEdition({ requestPolicy: "network-only" });
        setTimeout(() => setActionSuccess(null), 4000);
      }
    },
    [id, edition, mutCtx, reviewEdition, approveEdition, publishEdition, archiveEdition, refetchEdition]
  );

  const isEditable = edition ? (edition.status === "in_review" || edition.status === "draft") : false;

  if (fetching && !edition) {
    return <AdminLoader label="Loading edition..." />;
  }

  if (error || !edition) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-6xl mx-auto">
          <Alert variant="error">
            {error?.message || "Edition not found"}
          </Alert>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background">
      <div className="max-w-6xl mx-auto">
        <Tabs value={activeTab} onValueChange={setActiveTab}>
          {/* Header card with tabs at bottom edge */}
          <div className="bg-card border-b border-border px-6 pt-5 pb-0">
            <div className="flex items-start justify-between mb-4">
              <div className="flex items-start gap-3">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  render={<Link href="/admin/editions" />}
                  className="mt-0.5 text-muted-foreground hover:text-foreground"
                >
                  <ArrowLeft className="size-4" />
                </Button>
                <div>
                  <h1 className="text-lg font-semibold text-foreground leading-tight">
                    {edition.county.name} County
                  </h1>
                  <span className="text-sm text-muted-foreground">
                    {formatDateRange(edition.periodStart, edition.periodEnd)}
                  </span>
                </div>
              </div>
              <div className="flex items-center gap-3">
                {!isEditable && (
                  <span className="inline-flex items-center gap-1.5 rounded-full border border-amber-200 bg-amber-50 px-3 py-1 text-xs font-semibold text-amber-700">
                    <Lock className="size-3" />
                    Editing is locked
                  </span>
                )}
                <Select
                  value={edition.status}
                  disabled={statusChanging}
                  onValueChange={(val) => {
                    if (val && val !== edition.status) handleStatusChange(val);
                  }}
                >
                  <SelectTrigger className="h-7 w-auto min-w-0 gap-1 rounded-full px-2.5 text-xs font-medium">
                    <Badge variant={editionStatusVariant(edition.status)} className="pointer-events-none">
                      <SelectValue />
                    </Badge>
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={edition.status} disabled>
                      {EDITION_STATUS_LABELS[edition.status] ?? edition.status}
                    </SelectItem>
                    {(VALID_TRANSITIONS[edition.status] ?? []).map((status) => (
                      <SelectItem key={status} value={status}>
                        {EDITION_STATUS_LABELS[status] ?? status}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <Button
                  variant="outline"
                  size="sm"
                  render={
                    <a
                      href={`${process.env.NEXT_PUBLIC_WEB_APP_URL || "http://localhost:3001"}/preview/${edition.id}`}
                      target="_blank"
                      rel="noopener noreferrer"
                    />
                  }
                >
                  Preview
                  <ExternalLink className="size-3.5" />
                </Button>
              </div>
            </div>

            <TabsList variant="line">
              <TabsTrigger value="layout">Layout</TabsTrigger>
              <TabsTrigger value="posts">Posts</TabsTrigger>
              <TabsTrigger value="widgets">Widgets</TabsTrigger>
            </TabsList>
          </div>

          {actionError && (
            <Alert variant="error" className="mx-6 mt-4">
              <AlertDescription>{actionError}</AlertDescription>
            </Alert>
          )}
          {actionSuccess && (
            <Alert variant="success" className="mx-6 mt-4">
              <div className="flex items-center justify-between">
                <span>{actionSuccess}</span>
                <Button variant="ghost" size="xs" onClick={() => setActionSuccess(null)}>
                  dismiss
                </Button>
              </div>
            </Alert>
          )}

          <div className="px-6 pt-6">
            <TabsContent value="layout">
              <BroadsheetEditor
                edition={edition}
                refetchEdition={refetchEdition}
              />
            </TabsContent>

            <TabsContent value="posts">
              <EditionPostsView edition={edition} />
            </TabsContent>

            <TabsContent value="widgets">
              <EditionWidgetsView editionId={edition.id} />
            </TabsContent>

          </div>
        </Tabs>
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
  const [, moveSlot] = useMutation(MoveSlotMutation);
  const [, removePost] = useMutation(RemovePostFromEditionMutation);
  const [, changeSlotTemplate] = useMutation(ChangeSlotTemplateMutation);
  const [, reorderRows] = useMutation(ReorderEditionRowsMutation);
  const [, addRow] = useMutation(AddEditionRowMutation);
  const [, updateRowMut] = useMutation(UpdateEditionRowMutation);
  const [, deleteRowMut] = useMutation(DeleteEditionRowMutation);
  const [, addWidgetToEditionMut] = useMutation(AddWidgetToEditionMutation);
  const [, addSectionMut] = useMutation(AddSectionMutation);
  const [, updateSectionMut] = useMutation(UpdateSectionMutation);
  const [, deleteSectionMut] = useMutation(DeleteSectionMutation);
  const [, assignRowToSectionMut] = useMutation(AssignRowToSectionMutation);
  const [, reorderSectionsMut] = useMutation(ReorderSectionsMutation);

  const rowTemplates = rowTemplatesData?.rowTemplates ?? [];
  const postTemplates = postTemplatesData?.postTemplates ?? [];
  const sections = useMemo(
    () => edition ? [...edition.sections].sort((a, b) => a.sortOrder - b.sortOrder) : [],
    [edition]
  );

  // Row management
  const sortedRows = useMemo(
    () =>
      edition
        ? [...edition.rows].sort((a, b) => a.sortOrder - b.sortOrder)
        : [],
    [edition]
  );
  // DnD
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } })
  );

  const [activeRowId, setActiveRowId] = useState<string | null>(null);
  const [activeSectionId, setActiveSectionId] = useState<string | null>(null);
  const [dragType, setDragType] = useState<"row" | "section" | "slot" | null>(null);

  const handleDragStart = useCallback((event: DragStartEvent) => {
    const type = event.active.data.current?.type as "row" | "section" | "slot" | undefined;
    setDragType(type ?? null);
    if (type === "row") {
      setActiveRowId(event.active.id as string);
    } else if (type === "section") {
      setActiveSectionId(event.active.id as string);
    } else {
      setActiveSlotId(event.active.id as string);
    }
  }, []);

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const activeDragType = event.active.data.current?.type;
      const { active, over } = event;
      setDragType(null);

      // Section drag
      if (activeDragType === "section") {
        setActiveSectionId(null);
        if (!over || !edition || active.id === over.id) return;
        if (over.data.current?.type !== "section") return;
        const activeId = (active.id as string).replace("section-", "");
        const overId = (over.id as string).replace("section-", "");
        const oldIndex = sections.findIndex((s) => s.id === activeId);
        const newIndex = sections.findIndex((s) => s.id === overId);
        if (oldIndex < 0 || newIndex < 0) return;
        const newOrder = sections.map((s) => s.id);
        newOrder.splice(oldIndex, 1);
        newOrder.splice(newIndex, 0, activeId);
        await reorderSectionsMut({ editionId: edition.id, sectionIds: newOrder }, mutCtx);
        refetchEdition({ requestPolicy: "network-only" });
        return;
      }

      // Row drag
      if (activeDragType === "row") {
        setActiveRowId(null);
        if (!over || !edition || active.id === over.id) return;
        if (over.data.current?.type !== "row") return;
        const oldIndex = sortedRows.findIndex((r) => r.id === active.id);
        const newIndex = sortedRows.findIndex((r) => r.id === over.id);
        if (oldIndex < 0 || newIndex < 0) return;
        const newOrder = sortedRows.map((r) => r.id);
        newOrder.splice(oldIndex, 1);
        newOrder.splice(newIndex, 0, active.id as string);
        await reorderRows({ editionId: edition.id, rowIds: newOrder }, mutCtx);
        refetchEdition({ requestPolicy: "network-only" });
        return;
      }

      // Slot drag
      setActiveSlotId(null);
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
    [edition, sortedRows, sections, moveSlot, removePost, reorderRows, reorderSectionsMut, mutCtx, refetchEdition]
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

  const handleChangeRowTemplate = useCallback(
    async (rowId: string, rowTemplateSlug: string) => {
      await updateRowMut({ rowId, rowTemplateSlug }, mutCtx);
      refetchEdition({ requestPolicy: "network-only" });
    },
    [updateRowMut, mutCtx, refetchEdition]
  );

  const handleAddRow = useCallback(
    async (rowTemplateSlug: string, sortOrder?: number) => {
      const order = sortOrder ??
        (sortedRows.length > 0
          ? Math.max(...sortedRows.map((r) => r.sortOrder)) + 1
          : 0);
      await addRow(
        { editionId: edition!.id, rowTemplateSlug, sortOrder: order },
        mutCtx
      );
      refetchEdition({ requestPolicy: "network-only" });
    },
    [edition, sortedRows, addRow, mutCtx, refetchEdition]
  );

  const handleGenerate = useCallback(async () => {
    setActionError(null);
    setActionSuccess(null);
    const result = await generateEdition({ id }, mutCtx);
    if (result.error) {
      setActionError(result.error.message);
    } else {
      setActionSuccess("Layout regenerated");
      refetchEdition({ requestPolicy: "network-only" });
      setTimeout(() => setActionSuccess(null), 4000);
    }
  }, [id, mutCtx, generateEdition, refetchEdition]);

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

  const handleAddWidgetToEdition = useCallback(
    async (editionRowId: string, widgetId: string, slotIndex: number) => {
      const result = await addWidgetToEditionMut(
        { editionRowId, widgetId, slotIndex },
        mutCtx
      );
      if (result.error) {
        console.error("addWidgetToEdition failed:", result.error);
        setActionError(`Failed to add widget: ${result.error.message}`);
        return;
      }
      refetchEdition({ requestPolicy: "network-only" });
    },
    [addWidgetToEditionMut, mutCtx, refetchEdition]
  );

  // Section handlers
  const handleAddSection = useCallback(
    async (title: string, sortOrder?: number) => {
      const nextOrder = sortOrder ??
        (sections.length > 0
          ? Math.max(...sections.map((s) => s.sortOrder)) + 1
          : 0);
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
  const isDragging = activeSlotId !== null;
  const isDraggingRow = activeRowId !== null;
  const [allRowsCollapsed, setAllRowsCollapsed] = useState(false);

  const activeSlotData = activeSlotId
    ? sortedRows.flatMap((r) => r.slots).find((s) => s.id === activeSlotId)
    : null;

  const activeRowData = activeRowId
    ? sortedRows.find((r) => r.id === activeRowId)
    : null;

  const activeSectionData = activeSectionId
    ? sections.find((s) => s.id === activeSectionId.replace("section-", ""))
    : null;

  return (
    <>
      {actionError && (
        <Alert variant="error" className="mb-4">{actionError}</Alert>
      )}
      {actionSuccess && (
        <Alert variant="success" className="mb-4">
          <div className="flex items-center justify-between">
            <span>{actionSuccess}</span>
            <Button variant="ghost" size="xs" onClick={() => setActionSuccess(null)}>dismiss</Button>
          </div>
        </Alert>
      )}

      {/* Toolbar — view mode tabs + regenerate */}
      <div className="flex items-center justify-between mb-4">
        <div>
          {sortedRows.length > 0 && (
            <Tabs
              value={allRowsCollapsed ? "arrange" : "edit"}
              onValueChange={(v) => setAllRowsCollapsed(v === "arrange")}
            >
              <TabsList>
                <TabsTrigger value="arrange"><ListStart className="size-3.5 mr-1" />Structure</TabsTrigger>
                <TabsTrigger value="edit"><LayoutDashboard className="size-3.5 mr-1" />Posts</TabsTrigger>
              </TabsList>
            </Tabs>
          )}
        </div>
        {isEditable && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              const ok = window.confirm(
                "Regenerating will replace all rows, posts, sections, and widgets " +
                "with a fresh layout. Any manual edits to the broadsheet will be lost.\n\n" +
                "Continue?"
              );
              if (ok) handleGenerate();
            }}
          >
            Regenerate Layout
          </Button>
        )}
      </div>

      {/* Broadsheet layout with DnD */}
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
          {sortedRows.length === 0 ? (
            <div className="text-muted-foreground text-center py-12 bg-card rounded-lg border border-border">
              <p className="text-lg mb-2">Empty broadsheet</p>
              <p className="text-sm">
                Click &ldquo;Regenerate Layout&rdquo; to auto-populate, or add rows manually.
              </p>
            </div>
          ) : (
            <FlatRowLayout
              rows={sortedRows}
              isEditable={isEditable}
              isDragging={isDragging}
              dragType={dragType}
              allRowsCollapsed={allRowsCollapsed}
              rowTemplates={rowTemplates}
              postTemplates={postTemplates}
              onMoveRow={handleMoveRow}
              onDeleteRow={handleDeleteRow}
              onChangeRowTemplate={handleChangeRowTemplate}
              onChangeTemplate={handleChangeTemplate}
              onRemovePost={handleRemovePost}
              onViewPost={(postId) => router.push(`/admin/posts/${postId}`)}
              onAddRow={handleAddRow}
              onAddWidget={handleAddWidgetToEdition}
            />
          )}

        {isDragging && isEditable && <RemoveDropZone />}

        <DragOverlay>
          {activeSlotData ? <SlotCardOverlay slot={activeSlotData} /> : null}
          {activeRowData ? <RowDragOverlay row={activeRowData} /> : null}
          {activeSectionData ? <SectionDragOverlay section={activeSectionData} /> : null}
        </DragOverlay>
      </DndContext>

      {/* Inserter buttons are now inline between items — no bottom-of-page Add buttons */}
    </>
  );
}

// ─── Edition Posts View ─────────────────────────────────────────────────────

function EditionPostsView({ edition }: { edition: Edition }) {
  const router = useRouter();
  const [slottedFilter, setSlottedFilter] = useState<string>("all");

  // Fetch posts eligible for this edition with server-side slotted filtering.
  // Uses the same county-matching logic as the layout engine (locationables,
  // statewide tags, or no-location fallback) so the list matches what the
  // layout engine sees.
  const [{ data: postsData, fetching: postsFetching }] = useQuery({
    query: EditionPostsQuery,
    variables: {
      editionId: edition.id,
      slottedFilter: slottedFilter === "all" ? undefined : slottedFilter,
      limit: 200,
    },
  });

  const filteredPosts = postsData?.editionPosts?.posts ?? [];

  return (
    <>
      <div className="flex items-center gap-3 mb-4">
        <Tabs value={slottedFilter} onValueChange={setSlottedFilter}>
          <TabsList>
            <TabsTrigger value="all">All</TabsTrigger>
            <TabsTrigger value="slotted">Slotted</TabsTrigger>
            <TabsTrigger value="not_slotted">Not Slotted</TabsTrigger>
          </TabsList>
        </Tabs>
        <span className="text-sm text-muted-foreground ml-auto">
          {filteredPosts.length} post{filteredPosts.length !== 1 ? "s" : ""}
        </span>
      </div>

      {postsFetching ? (
        <AdminLoader />
      ) : filteredPosts.length === 0 ? (
        <div className="text-muted-foreground text-center py-12 text-sm">
          {slottedFilter === "not_slotted"
            ? "All matching posts are already slotted."
            : slottedFilter === "slotted"
              ? "No posts slotted in this edition yet."
              : "No active posts match this edition\u2019s county."}
        </div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Title</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Weight</TableHead>
              <TableHead>Status</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredPosts.map((post) => (
              <TableRow
                key={post.id}
                className="cursor-pointer"
                onClick={() => router.push(`/admin/posts/${post.id}`)}
              >
                <TableCell className="font-medium">{post.title}</TableCell>
                <TableCell>
                  <PostTypeBadge type={post.postType} />
                </TableCell>
                <TableCell>
                  {post.weight && <WeightBadge weight={post.weight} />}
                </TableCell>
                <TableCell>
                  <PostStatusBadge status={post.status} />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </>
  );
}

// ─── Edition Widgets View ─────────────────────────────────────────────────────

function EditionWidgetsView({ editionId }: { editionId: string }) {
  const router = useRouter();
  const [slottedFilter, setSlottedFilter] = useState<string>("all");

  const [{ data, fetching }] = useQuery({
    query: EditionWidgetsQuery,
    variables: {
      editionId,
      slottedFilter: slottedFilter === "all" ? undefined : slottedFilter,
      limit: 100,
    },
  });

  const widgets = data?.editionWidgets ?? [];

  return (
    <>
      <div className="flex items-center gap-3 mb-4">
        <Tabs value={slottedFilter} onValueChange={setSlottedFilter}>
          <TabsList>
            <TabsTrigger value="all">All</TabsTrigger>
            <TabsTrigger value="slotted">Slotted</TabsTrigger>
            <TabsTrigger value="not_slotted">Not Slotted</TabsTrigger>
          </TabsList>
        </Tabs>
        <span className="text-sm text-muted-foreground ml-auto">
          {widgets.length} widget{widgets.length !== 1 ? "s" : ""}
        </span>
      </div>

      {fetching ? (
        <AdminLoader />
      ) : widgets.length === 0 ? (
        <div className="text-muted-foreground text-center py-12 text-sm">
          {slottedFilter === "not_slotted"
            ? "All matching widgets are already slotted."
            : slottedFilter === "slotted"
              ? "No widgets slotted in this edition yet."
              : "No widgets match this edition\u2019s county and date range."}
        </div>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Type</TableHead>
              <TableHead>Summary</TableHead>
              <TableHead>County</TableHead>
              <TableHead>Date Range</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {widgets.map((w) => {
              const summary = widgetSummary(w.widgetType, w.data);
              return (
                <TableRow
                  key={w.id}
                  className="cursor-pointer"
                  onClick={() => router.push(`/admin/widgets/${w.id}`)}
                >
                  <TableCell>
                    <Badge
                      variant="secondary"
                      className={`text-[10px] ${WIDGET_TYPE_COLORS[w.widgetType] ?? ""}`}
                    >
                      {WIDGET_TYPE_LABELS[w.widgetType] ?? w.widgetType}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-muted-foreground truncate max-w-xs">
                    {summary || <span className="italic">Empty</span>}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {w.county?.name ?? "—"}
                  </TableCell>
                  <TableCell className="text-muted-foreground text-xs">
                    {w.startDate || w.endDate
                      ? `${w.startDate ?? "∞"} — ${w.endDate ?? "∞"}`
                      : "Evergreen"}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
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
  isDragging,
  dragType,
  collapsed,
  rowTemplates,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeRowTemplate,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddWidget,
}: {
  row: EditionRow;
  rowIndex: number;
  totalRows: number;
  isEditable: boolean;
  isDragging: boolean;
  dragType: "row" | "section" | "slot" | null;
  collapsed: boolean;
  rowTemplates: RowTemplate[];
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeRowTemplate: (rowId: string, slug: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddWidget: (editionRowId: string, widgetId: string, slotIndex: number) => void;
}) {
  const {
    attributes: sortableAttributes,
    listeners: sortableListeners,
    setNodeRef: setSortableRef,
    transform: sortableTransform,
    transition: sortableTransition,
    isDragging: isSortableDragging,
  } = useSortable({ id: row.id, data: { type: "row" }, disabled: !isEditable || !collapsed || dragType === "section" });

  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);
  const [templatePickerOpen, setTemplatePickerOpen] = useState(false);

  const sortableStyle = {
    transform: CSS.Transform.toString(sortableTransform),
    transition: sortableTransition,
  };

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

  const slotCount = row.slots.length;

  return (
    <div
      ref={setSortableRef}
      style={sortableStyle}
      className={`bg-card rounded-lg border border-border overflow-hidden ${
        isSortableDragging ? "opacity-30" : ""
      }`}
    >
      {/* Row header */}
      <div className="px-4 py-2.5 flex items-center justify-between">
        <div className="flex items-center gap-3">
          {isEditable && collapsed && (
            <button
              className="cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground shrink-0"
              {...sortableAttributes}
              {...sortableListeners}
              tabIndex={-1}
            >
              <GripVertical className="size-4" />
            </button>
          )}
          <span className="text-xs font-mono text-muted-foreground bg-background rounded px-1.5 py-0.5">
            {rowIndex + 1}
          </span>
          {isEditable && rowTemplates.length > 0 ? (
            <button
              className="text-sm font-semibold text-foreground hover:underline truncate"
              onClick={() => setTemplatePickerOpen(true)}
            >
              {row.rowTemplate.displayName}
            </button>
          ) : (
            <span className="text-sm font-semibold text-foreground truncate">
              {row.rowTemplate.displayName}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {collapsed && (
            <span className="text-xs text-muted-foreground">
              {slotCount} slot{slotCount !== 1 ? "s" : ""}
            </span>
          )}
          {isEditable && (
            <>
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={() => setConfirmDeleteOpen(true)}
                className="ml-2 hover:text-destructive"
                title="Delete row"
              >
                <X className="size-4" />
              </Button>
              <ConfirmDialog
                open={confirmDeleteOpen}
                onOpenChange={setConfirmDeleteOpen}
                title="Delete row"
                description="Posts in this row's slots will be unassigned from the edition and returned to the post pool. No posts are deleted — they can be reassigned to other slots."
                confirmLabel="Delete row"
                onConfirm={() => onDeleteRow(row.id)}
              />
            </>
          )}
          <RowTemplatePickerDialog
            open={templatePickerOpen}
            onOpenChange={setTemplatePickerOpen}
            templates={rowTemplates}
            currentSlug={row.rowTemplate.slug}
            onSelect={(slug) => {
              onChangeRowTemplate(row.id, slug);
              setTemplatePickerOpen(false);
            }}
          />
        </div>
      </div>

      {!collapsed && (
        <div className="px-4 pt-0 pb-4">
          <div className="grid grid-cols-3 gap-3">
            {templateSlots.map((tSlot) => (
              <SlotCell
                key={tSlot.slotIndex}
                rowId={row.id}
                templateSlot={tSlot}
                editionSlots={slotsByIndex.get(tSlot.slotIndex) ?? []}
                isEditable={isEditable}
                isDragging={isDragging}
                postTemplates={postTemplates}
                onChangeTemplate={onChangeTemplate}
                onRemovePost={onRemovePost}
                onViewPost={onViewPost}
                onAddWidget={(widgetId) => onAddWidget(row.id, widgetId, tSlot.slotIndex)}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ─── FlatRowLayout ───────────────────────────────────────────────────────────
// Renders all rows in sort_order without section grouping. Sections are kept
// as advisory metadata in the DB but don't affect the admin layout anymore.
// Visual section breaks come from SectionSep widgets placed by the layout engine.

function FlatRowLayout({
  rows,
  isEditable,
  isDragging,
  dragType,
  allRowsCollapsed,
  rowTemplates,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeRowTemplate,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddRow,
  onAddWidget,
}: {
  rows: EditionRow[];
  isEditable: boolean;
  isDragging: boolean;
  dragType: "row" | "section" | "slot" | null;
  allRowsCollapsed: boolean;
  rowTemplates: RowTemplate[];
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeRowTemplate: (rowId: string, slug: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddRow: (templateSlug: string, sortOrder?: number) => void;
  onAddWidget: (editionRowId: string, widgetId: string, slotIndex: number) => void;
}) {
  const sortedRows = [...rows].sort((a, b) => a.sortOrder - b.sortOrder);
  const rowIds = sortedRows.map((r) => r.id);

  const getInsertSortOrder = (index: number) => {
    if (sortedRows.length === 0) return 0;
    if (index <= 0) return (sortedRows[0]?.sortOrder ?? 0) - 10;
    if (index >= sortedRows.length) return (sortedRows[sortedRows.length - 1]?.sortOrder ?? 0) + 10;
    const prev = sortedRows[index - 1].sortOrder;
    const next = sortedRows[index].sortOrder;
    return Math.floor((prev + next) / 2);
  };

  return (
    <SortableContext items={rowIds} strategy={verticalListSortingStrategy}>
      <div className="space-y-3">
        {sortedRows.map((row, idx) => (
          <div key={row.id}>
            {isEditable && allRowsCollapsed && (
              <InlineInserter
                sortOrder={getInsertSortOrder(idx)}
                rowTemplates={rowTemplates}
                onAddRow={onAddRow}
              />
            )}
            <RowEditor
              row={row}
              rowIndex={idx}
              totalRows={rows.length}
              isEditable={isEditable}
              isDragging={isDragging}
              dragType={dragType}
              collapsed={allRowsCollapsed}
              rowTemplates={rowTemplates}
              postTemplates={postTemplates}
              onMoveRow={onMoveRow}
              onDeleteRow={onDeleteRow}
              onChangeRowTemplate={onChangeRowTemplate}
              onChangeTemplate={onChangeTemplate}
              onRemovePost={onRemovePost}
              onViewPost={onViewPost}
              onAddWidget={onAddWidget}
            />
          </div>
        ))}
        {isEditable && allRowsCollapsed && (
          <InlineInserter
            sortOrder={getInsertSortOrder(sortedRows.length)}
            rowTemplates={rowTemplates}
            onAddRow={onAddRow}
          />
        )}
      </div>
    </SortableContext>
  );
}

// ─── SectionGroupedLayout (legacy, kept for reference) ───────────────────────

function SectionGroupedLayout({
  rows,
  sections,
  isEditable,
  isDragging,
  dragType,
  allRowsCollapsed,
  rowTemplates,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeRowTemplate,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddRow,
  onAddWidget,
  onAddSection,
  onUpdateSection,
  onDeleteSection,
}: {
  rows: EditionRow[];
  sections: EditionSection[];
  isEditable: boolean;
  isDragging: boolean;
  dragType: "row" | "section" | "slot" | null;
  allRowsCollapsed: boolean;
  rowTemplates: RowTemplate[];
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeRowTemplate: (rowId: string, slug: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddRow: (templateSlug: string, sortOrder?: number) => void;
  onAddWidget: (editionRowId: string, widgetId: string, slotIndex: number) => void;
  onAddSection: (title: string, sortOrder?: number) => void;
  onUpdateSection: (sectionId: string, title: string) => void;
  onDeleteSection: (sectionId: string) => void;
}) {
  // Build row items per section (and ungrouped)
  const buildItems = useCallback(
    (sectionId: string | null): EditionRow[] => {
      return rows
        .filter((r) => (sectionId ? r.sectionId === sectionId : !r.sectionId))
        .sort((a, b) => a.sortOrder - b.sortOrder);
    },
    [rows]
  );

  const ungroupedItems = useMemo(() => buildItems(null), [buildItems]);

  // IDs for per-group SortableContexts
  const ungroupedRowIds = useMemo(
    () => ungroupedItems.map((r) => r.id),
    [ungroupedItems]
  );
  const sectionSortableIds = useMemo(
    () => sections.map((s) => `section-${s.id}`),
    [sections]
  );

  // Calculate sort order for inserting between rows
  const getInsertSortOrder = (items: EditionRow[], index: number) => {
    if (items.length === 0) return 0;
    if (index <= 0) return (items[0]?.sortOrder ?? 0) - 10;
    if (index >= items.length) return (items[items.length - 1]?.sortOrder ?? 0) + 10;
    const prev = items[index - 1].sortOrder;
    const next = items[index].sortOrder;
    return Math.floor((prev + next) / 2);
  };

  const renderLayoutItems = (items: EditionRow[], sectionId: string | null) => (
    <div className="space-y-3">
      {items.map((row, idx) => (
        <div key={row.id}>
          {isEditable && allRowsCollapsed && (
            <InlineInserter
              sortOrder={getInsertSortOrder(items, idx)}
              rowTemplates={rowTemplates}
              onAddRow={onAddRow}
            />
          )}
          <RowEditor
            row={row}
            rowIndex={idx}
            totalRows={rows.length}
            isEditable={isEditable}
            isDragging={isDragging}
            dragType={dragType}
            collapsed={allRowsCollapsed}
            rowTemplates={rowTemplates}
            postTemplates={postTemplates}
            onMoveRow={onMoveRow}
            onDeleteRow={onDeleteRow}
            onChangeRowTemplate={onChangeRowTemplate}
            onChangeTemplate={onChangeTemplate}
            onRemovePost={onRemovePost}
            onViewPost={onViewPost}
            onAddWidget={onAddWidget}
          />
        </div>
      ))}
      {/* Trailing inserter after last item */}
      {isEditable && allRowsCollapsed && (
        <InlineInserter
          sortOrder={getInsertSortOrder(items, items.length)}
          rowTemplates={rowTemplates}
          onAddRow={onAddRow}
        />
      )}
    </div>
  );

  return (
    <div className="space-y-4">
      {ungroupedItems.length > 0 && (
        <div>
          <div className="flex items-center gap-2 px-1 mb-2">
            <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
              Above the Fold
            </span>
            <span className="text-xs text-muted-foreground/50">
              ({ungroupedItems.length} item{ungroupedItems.length !== 1 ? "s" : ""})
            </span>
          </div>
          <SortableContext items={ungroupedRowIds} strategy={verticalListSortingStrategy}>
            {renderLayoutItems(ungroupedItems, null)}
          </SortableContext>
        </div>
      )}

      {/* Fold indicator */}
      {ungroupedItems.length > 0 && sections.length > 0 && (
        <div className="flex items-center gap-3 py-1">
          <div className="flex-1 border-t border-dashed border-muted-foreground/30" />
          <span className="text-[10px] font-semibold uppercase tracking-widest text-muted-foreground/50">
            Fold
          </span>
          <div className="flex-1 border-t border-dashed border-muted-foreground/30" />
        </div>
      )}

      {/* Sections — each in its own SortableContext for section-level reordering */}
      <SortableContext items={sectionSortableIds} strategy={verticalListSortingStrategy}>
        {/* Section inserter before first section */}
        {isEditable && allRowsCollapsed && sections.length > 0 && (
          <SectionInserter
            sortOrder={(sections[0]?.sortOrder ?? 0) - 10}
            onAddSection={onAddSection}
          />
        )}

        {sections.map((section, sIdx) => {
          const sectionItems = buildItems(section.id);
          return (
            <div key={section.id}>
              <SectionBlock
                section={section}
                items={sectionItems}
                rows={rows}
                isEditable={isEditable}
                isDragging={isDragging}
                dragType={dragType}
                allRowsCollapsed={allRowsCollapsed}
                rowTemplates={rowTemplates}
                postTemplates={postTemplates}
                onMoveRow={onMoveRow}
                onDeleteRow={onDeleteRow}
                onChangeRowTemplate={onChangeRowTemplate}
                onChangeTemplate={onChangeTemplate}
                onRemovePost={onRemovePost}
                onViewPost={onViewPost}
                onAddRow={onAddRow}
                onAddWidget={onAddWidget}
                onUpdateSection={onUpdateSection}
                onDeleteSection={onDeleteSection}
              />
              {/* Section inserter after each section */}
              {isEditable && allRowsCollapsed && (
                <SectionInserter
                  sortOrder={
                    sIdx < sections.length - 1
                      ? Math.floor((section.sortOrder + sections[sIdx + 1].sortOrder) / 2)
                      : section.sortOrder + 10
                  }
                  onAddSection={onAddSection}
                />
              )}
            </div>
          );
        })}
      </SortableContext>

      {/* If no sections yet and no ungrouped items, show initial inserters */}
      {isEditable && allRowsCollapsed && sections.length === 0 && ungroupedItems.length === 0 && (
        <div className="space-y-2">
          <InlineInserter
            sortOrder={0}
            rowTemplates={rowTemplates}
            onAddRow={onAddRow}
          />
          <SectionInserter sortOrder={0} onAddSection={onAddSection} />
        </div>
      )}
    </div>
  );
}

// ─── SectionBlock ────────────────────────────────────────────────────────────

function SectionBlock({
  section,
  items,
  rows,
  isEditable,
  isDragging,
  dragType,
  allRowsCollapsed,
  rowTemplates,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeRowTemplate,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddRow,
  onAddWidget,
  onUpdateSection,
  onDeleteSection,
}: {
  section: EditionSection;
  items: EditionRow[];
  rows: EditionRow[];
  isEditable: boolean;
  isDragging: boolean;
  dragType: "row" | "section" | "slot" | null;
  allRowsCollapsed: boolean;
  rowTemplates: RowTemplate[];
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeRowTemplate: (rowId: string, slug: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddRow: (templateSlug: string, sortOrder?: number) => void;
  onAddWidget: (editionRowId: string, widgetId: string, slotIndex: number) => void;
  onUpdateSection: (sectionId: string, title: string) => void;
  onDeleteSection: (sectionId: string) => void;
}) {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState(section.title);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState(false);

  const sortableId = `section-${section.id}`;
  const {
    attributes: sortableAttributes,
    listeners: sortableListeners,
    setNodeRef: setSortableRef,
    transform: sortableTransform,
    transition: sortableTransition,
    isDragging: isSortableDragging,
  } = useSortable({ id: sortableId, data: { type: "section" }, disabled: !isEditable || !allRowsCollapsed || dragType === "row" || dragType === "slot" });

  const sortableStyle = {
    transform: CSS.Transform.toString(sortableTransform),
    transition: sortableTransition,
  };

  const handleSave = () => {
    onUpdateSection(section.id, editTitle);
    setIsEditing(false);
  };

  // Row IDs in this section for SortableContext
  const sectionRowIds = useMemo(
    () => items.map((r) => r.id),
    [items]
  );

  const getInsertSortOrder = (index: number) => {
    if (items.length === 0) return 0;
    if (index <= 0) return (items[0]?.sortOrder ?? 0) - 10;
    if (index >= items.length) return (items[items.length - 1]?.sortOrder ?? 0) + 10;
    const prev = items[index - 1].sortOrder;
    const next = items[index].sortOrder;
    return Math.floor((prev + next) / 2);
  };

  return (
    <div
      ref={setSortableRef}
      style={sortableStyle}
      className={`rounded-lg bg-amber-100/60 overflow-hidden ${isSortableDragging ? "opacity-30" : ""}`}
    >
      {/* Section header */}
      <div className="flex items-center gap-3 px-3 py-2.5">
        {allRowsCollapsed && isEditable && (
          <button
            className="cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground shrink-0"
            {...sortableAttributes}
            {...sortableListeners}
            tabIndex={-1}
          >
            <GripVertical className="size-4" />
          </button>
        )}
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="text-muted-foreground w-5"
        >
          {isCollapsed ? <ChevronRight className="size-4" /> : <ChevronDown className="size-4" />}
        </Button>

        {isEditing ? (
          <div className="flex items-center gap-2 flex-1">
            <Input
              value={editTitle}
              onChange={(e) => setEditTitle(e.target.value)}
              className="h-7 text-sm font-semibold flex-1 max-w-xs"
              placeholder="Section title"
              autoFocus
            />
            <Button variant="admin" size="xs" onClick={handleSave}>
              Save
            </Button>
            <Button variant="ghost" size="xs" onClick={() => setIsEditing(false)}>
              Cancel
            </Button>
          </div>
        ) : (
          <div className="flex items-center gap-2 flex-1">
            <span className="text-sm font-semibold text-foreground">
              {section.title}
            </span>
            {section.topicSlug && (
              <Badge variant="warning" className="text-[10px]">{section.topicSlug}</Badge>
            )}
            <span className="text-xs text-muted-foreground/50">
              ({items.length} item{items.length !== 1 ? "s" : ""})
            </span>
          </div>
        )}

        {isEditable && !isEditing && (
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="xs"
              onClick={() => {
                setEditTitle(section.title);
                setIsEditing(true);
              }}
            >
              Edit
            </Button>
            <Button
              variant="ghost"
              size="xs"
              className="text-destructive hover:text-destructive"
              onClick={() => setConfirmDeleteOpen(true)}
            >
              Delete
            </Button>
            <ConfirmDialog
              open={confirmDeleteOpen}
              onOpenChange={setConfirmDeleteOpen}
              title={`Delete section "${section.title}"`}
              description="Rows and widgets in this section will become ungrouped and appear above the fold. No rows, widgets, or posts are deleted — only the section grouping is removed."
              confirmLabel="Delete section"
              onConfirm={() => onDeleteSection(section.id)}
            />
          </div>
        )}
      </div>

      {/* Section content (collapsible) */}
      {!isCollapsed && (
        <div className="p-3">
          {items.length === 0 && !isEditable ? (
            <div className="text-xs text-muted-foreground py-4 text-center">
              No items in this section. Drag rows here or add items.
            </div>
          ) : (
            <SortableContext items={sectionRowIds} strategy={verticalListSortingStrategy}>
              <div className="space-y-3">
                {items.map((row, idx) => (
                  <div key={row.id}>
                    {isEditable && allRowsCollapsed && (
                      <InlineInserter
                        sortOrder={getInsertSortOrder(idx)}
                        rowTemplates={rowTemplates}
                        onAddRow={onAddRow}
                      />
                    )}
                    <RowEditor
                      row={row}
                      rowIndex={idx}
                      totalRows={rows.length}
                      isEditable={isEditable}
                      isDragging={isDragging}
                      dragType={dragType}
                      collapsed={allRowsCollapsed}
                      rowTemplates={rowTemplates}
                      postTemplates={postTemplates}
                      onMoveRow={onMoveRow}
                      onDeleteRow={onDeleteRow}
                      onChangeRowTemplate={onChangeRowTemplate}
                      onChangeTemplate={onChangeTemplate}
                      onRemovePost={onRemovePost}
                      onViewPost={onViewPost}
                      onAddWidget={onAddWidget}
                    />
                  </div>
                ))}
                {isEditable && allRowsCollapsed && (
                  <InlineInserter
                    sortOrder={getInsertSortOrder(items.length)}
                    rowTemplates={rowTemplates}
                    onAddRow={onAddRow}
                  />
                )}
              </div>
            </SortableContext>
          )}
        </div>
      )}
    </div>
  );
}

// ─── InlineInserter (thin row between layout items) ─────────────────────────

function InlineInserter({
  sortOrder,
  rowTemplates,
  onAddRow,
}: {
  sortOrder: number;
  rowTemplates: RowTemplate[];
  onAddRow: (templateSlug: string, sortOrder?: number) => void;
}) {
  const [templatePickerOpen, setTemplatePickerOpen] = useState(false);

  return (
    <div className="group relative flex items-center my-1">
      <div className="absolute inset-0 flex items-center">
        <div className="w-full border-t border-dashed border-border" />
      </div>
      <div className="relative mx-auto flex items-center gap-1 opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="xs"
          className="text-[10px] text-muted-foreground h-5 px-1.5 bg-background"
          onClick={() => setTemplatePickerOpen(true)}
        >
          <Plus className="size-3" />
          Row
        </Button>
      </div>

      <RowTemplatePickerDialog
        open={templatePickerOpen}
        onOpenChange={setTemplatePickerOpen}
        templates={rowTemplates}
        currentSlug={null}
        onSelect={(slug) => {
          onAddRow(slug, sortOrder);
          setTemplatePickerOpen(false);
        }}
      />
    </div>
  );
}

// ─── SectionInserter (thin line between sections) ───────────────────────────

function SectionInserter({
  sortOrder,
  onAddSection,
}: {
  sortOrder: number;
  onAddSection: (title: string, sortOrder?: number) => void;
}) {
  const [title, setTitle] = useState("");

  return (
    <div className="group relative flex items-center my-1">
      <div className="absolute inset-0 flex items-center">
        <div className="w-full border-t border-dashed border-amber-300/60" />
      </div>
      <div className="relative mx-auto flex items-center opacity-0 group-hover:opacity-100 focus-within:opacity-100 transition-opacity">
        <Popover>
          <PopoverTrigger
            render={
              <Button
                variant="ghost"
                size="xs"
                className="text-[10px] text-amber-600 h-5 px-1.5 bg-background"
              />
            }
          >
            <Plus className="size-3" />
            Section
          </PopoverTrigger>
          <PopoverContent className="w-56 p-3" align="center">
            <div className="space-y-2">
              <label className="text-xs font-medium text-muted-foreground">Section name</label>
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="h-8 text-sm"
                placeholder="e.g. Public Safety"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === "Enter" && title.trim()) {
                    onAddSection(title.trim(), sortOrder);
                    setTitle("");
                  }
                }}
              />
              <Button
                variant="admin"
                size="xs"
                className="w-full"
                disabled={!title.trim()}
                onClick={() => {
                  onAddSection(title.trim(), sortOrder);
                  setTitle("");
                }}
              >
                Add Section
              </Button>
            </div>
          </PopoverContent>
        </Popover>
      </div>
    </div>
  );
}

// ─── AddSectionButton (legacy, kept for empty state) ────────────────────────

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
      <Button
        variant="outline"
        className="flex-1 py-3 h-auto border-2 border-dashed border-amber-300 text-sm font-medium text-amber-600 hover:border-amber-400 hover:text-amber-700"
        onClick={() => setIsOpen(true)}
      >
        <Plus className="size-3.5" />
        Add Section
      </Button>
    );
  }

  return (
    <div className="flex-1 flex items-center gap-2 py-2 px-3 rounded-lg border border-amber-300 bg-amber-50/50">
      <Input
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
        placeholder="Section title..."
        className="text-sm flex-1 h-8 bg-transparent border-none shadow-none"
        autoFocus
      />
      <Button
        variant="admin"
        size="xs"
        onClick={handleSubmit}
        disabled={!title.trim()}
      >
        Add
      </Button>
      <Button
        variant="ghost"
        size="xs"
        onClick={() => { setIsOpen(false); setTitle(""); }}
      >
        Cancel
      </Button>
    </div>
  );
}

// ─── SlotCell (droppable grid cell) ──────────────────────────────────────────

function SlotCell({
  rowId,
  templateSlot,
  editionSlots,
  isEditable,
  isDragging,
  postTemplates,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddWidget,
}: {
  rowId: string;
  templateSlot: TemplateSlotDef;
  editionSlots: EditionSlot[];
  isEditable: boolean;
  isDragging: boolean;
  postTemplates: PostTemplate[];
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddWidget: (widgetId: string) => void;
}) {
  const droppableId = `drop-${rowId}-${templateSlot.slotIndex}`;
  const { isOver, setNodeRef } = useDroppable({
    id: droppableId,
    disabled: !isEditable,
  });
  const colSpan = WEIGHT_SPAN[templateSlot.weight] ?? 1;
  const hasRoom = editionSlots.length < templateSlot.count;
  const [widgetPickerOpen, setWidgetPickerOpen] = useState(false);

  return (
    <div
      ref={setNodeRef}
      className={`space-y-2 rounded-lg p-2 transition-colors min-h-[80px] ${
        isOver
          ? "bg-amber-50 ring-2 ring-amber-300"
          : isDragging && hasRoom && isEditable
            ? "ring-1 ring-amber-200 bg-amber-50/30"
            : hasRoom && isEditable
              ? "bg-muted/30"
              : ""
      }`}
      style={{ gridColumn: `span ${colSpan}` }}
    >
      {editionSlots.map((slot) =>
        slot.kind === "widget" && slot.widget ? (
          <WidgetSlotCard key={slot.id} slot={slot} isEditable={isEditable} onRemovePost={onRemovePost} />
        ) : slot.post ? (
          <DraggableSlotCard
            key={slot.id}
            slot={slot}
            isEditable={isEditable}
            postTemplates={postTemplates}
            onChangeTemplate={onChangeTemplate}
            onRemovePost={onRemovePost}
            onViewPost={onViewPost}
          />
        ) : null
      )}
      {hasRoom && (
        <div
          className={`rounded-lg border-2 border-dashed p-3 flex flex-col items-center justify-center gap-2 ${
            isOver
              ? "border-amber-400 bg-amber-50/50"
              : isDragging && isEditable
                ? "border-amber-300"
                : "border-border"
          }`}
        >
          {isOver ? (
            <span className="text-xs font-medium text-amber-600">Drop here</span>
          ) : (
            <>
              <div className="flex items-center gap-2">
                <WeightBadge weight={templateSlot.weight} />
                <span className="text-xs text-muted-foreground">
                  {templateSlot.count - editionSlots.length} open
                </span>
              </div>
              {isEditable && (
                <Button
                  variant="ghost"
                  size="xs"
                  className="text-[10px] text-violet-600 hover:text-violet-700 h-5 px-1.5"
                  onClick={() => setWidgetPickerOpen(true)}
                >
                  <Puzzle className="size-3 mr-1" />
                  Add Widget
                </Button>
              )}
            </>
          )}
        </div>
      )}
      <WidgetPickerDialog
        open={widgetPickerOpen}
        onOpenChange={setWidgetPickerOpen}
        onSelect={(widgetId) => {
          onAddWidget(widgetId);
          setWidgetPickerOpen(false);
        }}
      />
    </div>
  );
}

// ─── WidgetPickerDialog (search & select existing widget) ────────────────────

const WIDGET_TYPE_LABELS: Record<string, string> = {
  number: "Number",
  stat_card: "Number",
  number_block: "Number",
  pull_quote: "Pull Quote",
  resource_bar: "Resource Bar",
  weather: "Weather",
  section_sep: "Section Sep",
};

const WIDGET_TYPE_COLORS: Record<string, string> = {
  number: "bg-amber-100 text-amber-800",
  stat_card: "bg-amber-100 text-amber-800",
  number_block: "bg-amber-100 text-amber-800",
  pull_quote: "bg-rose-100 text-rose-800",
  resource_bar: "bg-teal-100 text-teal-800",
  weather: "bg-sky-100 text-sky-800",
  section_sep: "bg-gray-100 text-gray-700",
};

function widgetSummary(widgetType: string, dataStr: string | null): string {
  if (!dataStr) return "";
  try {
    const data = typeof dataStr === "string" ? JSON.parse(dataStr) : dataStr;
    switch (widgetType) {
      case "number":
      case "stat_card":
      case "number_block":
        return [data.number, data.title || data.label].filter(Boolean).join(" — ");
      case "pull_quote":
        return data.quote
          ? `"${data.quote.slice(0, 50)}${data.quote.length > 50 ? "..." : ""}"`
          : "";
      case "resource_bar":
        return data.label || "";
      case "weather":
        return data.config?.location || data.variant || "";
      case "section_sep":
        return data.title || "";
      default:
        return "";
    }
  } catch {
    return "";
  }
}

function WidgetPickerDialog({
  open,
  onOpenChange,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (widgetId: string) => void;
}) {
  const params = useParams();
  const editionId = params.id as string;
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [{ data, fetching }] = useQuery({
    query: EditionWidgetsQuery,
    variables: {
      editionId,
      slottedFilter: "not_slotted",
      limit: 50,
    },
    pause: !open,
  });

  const allWidgets = data?.editionWidgets ?? [];
  const widgets = typeFilter === "all"
    ? allWidgets
    : allWidgets.filter((w) => w.widgetType === typeFilter);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Add Widget to Slot</DialogTitle>
          <DialogDescription>Select an existing widget to place in this slot.</DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <Select value={typeFilter} onValueChange={(v) => v && setTypeFilter(v)}>
            <SelectTrigger className="w-48">
              <SelectValue placeholder="All types" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All types</SelectItem>
              {Object.entries(WIDGET_TYPE_LABELS).map(([type, label]) => (
                <SelectItem key={type} value={type}>
                  {label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {fetching ? (
            <div className="py-8 text-center text-sm text-muted-foreground">Loading widgets...</div>
          ) : widgets.length === 0 ? (
            <div className="py-8 text-center text-sm text-muted-foreground">
              No widgets found.{" "}
              <Link href="/admin/widgets" className="text-violet-600 hover:underline">
                Create one
              </Link>
            </div>
          ) : (
            <div className="max-h-[320px] overflow-y-auto space-y-1">
              {widgets.map((w) => {
                const summary = widgetSummary(w.widgetType, w.data);
                return (
                  <button
                    key={w.id}
                    className="w-full flex items-center gap-3 p-2.5 rounded-lg hover:bg-muted/50 transition-colors text-left"
                    onClick={() => onSelect(w.id)}
                  >
                    <Badge
                      variant="secondary"
                      className={`text-[10px] shrink-0 ${WIDGET_TYPE_COLORS[w.widgetType] ?? ""}`}
                    >
                      {WIDGET_TYPE_LABELS[w.widgetType] ?? w.widgetType}
                    </Badge>
                    <span className="text-sm text-muted-foreground truncate flex-1">
                      {summary || <span className="italic">Empty</span>}
                    </span>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ─── WidgetSlotCard (for widget slots in rows) ──────────────────────────────

function WidgetSlotCard({
  slot,
  isEditable,
  onRemovePost,
}: {
  slot: EditionSlot;
  isEditable: boolean;
  onRemovePost: (slotId: string) => void;
}) {
  const widget = slot.widget!;
  const [confirmOpen, setConfirmOpen] = useState(false);
  const widgetLabel = widget.widgetType
    .split("_")
    .map((w: string) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");

  return (
    <div className="rounded-lg border border-violet-200 bg-violet-50/50 p-3">
      <div className="flex items-start gap-2">
        <div className="flex-1 min-w-0">
          <Link
            href={`/admin/widgets/${widget.id}`}
            className="block text-sm font-medium text-foreground truncate hover:underline"
          >
            {widgetLabel}
          </Link>
          <div className="flex items-center gap-1.5 mt-1">
            <Badge variant="secondary" className="text-[10px] bg-violet-100 text-violet-800">
              widget
            </Badge>
            <span className="text-[10px] text-muted-foreground">{widget.authoringMode}</span>
          </div>
        </div>
        {isEditable && (
          <>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={() => setConfirmOpen(true)}
              className="text-muted-foreground hover:text-destructive shrink-0"
              title="Remove widget from slot"
            >
              <X className="size-3.5" />
            </Button>
            <ConfirmDialog
              open={confirmOpen}
              onOpenChange={setConfirmOpen}
              title="Remove widget from slot"
              description="The widget will be removed from this row but not deleted. You can add it back later."
              confirmLabel="Remove"
              onConfirm={() => onRemovePost(slot.id)}
            />
          </>
        )}
      </div>
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
  const [confirmRemoveOpen, setConfirmRemoveOpen] = useState(false);
  const [templatePickerOpen, setTemplatePickerOpen] = useState(false);

  const style = transform
    ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }
    : undefined;

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`rounded-lg border border-border bg-card p-3 transition-shadow ${
        isDragging ? "opacity-30 shadow-lg" : "hover:shadow-md"
      }`}
    >
      <div className="flex items-start gap-2">
        {isEditable && (
          <button
            className="mt-0.5 cursor-grab active:cursor-grabbing text-muted-foreground hover:text-foreground shrink-0"
            {...attributes}
            {...listeners}
            tabIndex={-1}
          >
            <GripVertical className="size-3.5" />
          </button>
        )}
        <div className="flex-1 min-w-0">
          <button
            className="block w-full text-sm font-medium text-foreground truncate text-left hover:underline"
            onClick={() => slot.post && onViewPost(slot.post.id)}
          >
            {slot.post?.title ?? "Untitled"}
          </button>
          <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
            {slot.post && <PostTypeBadge type={slot.post.postType} />}
            {slot.post && <WeightBadge weight={slot.post.weight} />}
            {isEditable && postTemplates.length > 0 && slot.postTemplate && (
              <Button
                variant="ghost"
                size="xs"
                className="h-5 text-[10px] px-1.5 border border-border text-muted-foreground"
                onClick={() => setTemplatePickerOpen(true)}
              >
                {postTemplates.find((pt) => pt.slug === slot.postTemplate)?.displayName ?? slot.postTemplate}
              </Button>
            )}
            {!isEditable && slot.postTemplate && (
              <span className="text-[10px] text-muted-foreground">{slot.postTemplate}</span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-0.5 shrink-0">
          {isEditable && (
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={() => setConfirmRemoveOpen(true)}
              className="text-muted-foreground hover:text-destructive"
              title="Remove from slot"
            >
              <X className="size-3.5" />
            </Button>
          )}
        </div>
      </div>

      <PostTemplatePickerDialog
        open={templatePickerOpen}
        onOpenChange={setTemplatePickerOpen}
        templates={postTemplates}
        currentSlug={slot.postTemplate}
        onSelect={(slug) => {
          onChangeTemplate(slot.id, slug);
          setTemplatePickerOpen(false);
        }}
      />
      <ConfirmDialog
        open={confirmRemoveOpen}
        onOpenChange={setConfirmRemoveOpen}
        title="Remove post from slot"
        description={`Remove "${slot.post?.title ?? "this item"}" from this slot? The post returns to the unassigned pool and can be placed in another slot. It is not deleted.`}
        confirmLabel="Remove post"
        onConfirm={() => onRemovePost(slot.id)}
      />
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
          : "border-border bg-muted/50 text-muted-foreground"
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
  if (slot.kind === "widget" && slot.widget) {
    return (
      <div className="rounded-lg border border-violet-300 bg-card shadow-xl p-3 max-w-xs rotate-1 scale-[1.02]">
        <div className="text-sm font-medium text-foreground truncate">
          {slot.widget.widgetType.replace(/_/g, " ")}
        </div>
        <Badge variant="secondary" className="text-[10px] mt-1 bg-violet-100 text-violet-800">widget</Badge>
      </div>
    );
  }
  return (
    <div className="rounded-lg border border-amber-300 bg-card shadow-xl p-3 max-w-xs rotate-1 scale-[1.02]">
      <div className="text-sm font-medium text-foreground truncate">
        {slot.post?.title ?? "Untitled"}
      </div>
      <div className="flex items-center gap-1.5 mt-1">
        {slot.post && <PostTypeBadge type={slot.post.postType} />}
        {slot.post && <WeightBadge weight={slot.post.weight} />}
      </div>
    </div>
  );
}

// ─── RowDragOverlay (row drag ghost) ──────────────────────────────────────────

function RowDragOverlay({ row }: { row: EditionRow }) {
  return (
    <div className="rounded-lg border border-amber-300 bg-card shadow-xl px-4 py-2.5 max-w-md rotate-1 scale-[1.02]">
      <div className="flex items-center gap-3">
        <GripVertical className="size-4 text-muted-foreground" />
        <span className="text-sm font-semibold text-foreground">
          {row.rowTemplate.displayName}
        </span>
        <span className="text-xs text-muted-foreground">
          {row.slots.length} post{row.slots.length !== 1 ? "s" : ""}
        </span>
      </div>
    </div>
  );
}

// ─── SectionDragOverlay (section drag ghost) ─────────────────────────────────

function SectionDragOverlay({ section }: { section: EditionSection }) {
  return (
    <div className="rounded-lg border border-amber-300 bg-amber-50 shadow-xl px-4 py-2.5 max-w-md rotate-1 scale-[1.02]">
      <div className="flex items-center gap-3">
        <GripVertical className="size-4 text-amber-600" />
        <span className="text-sm font-semibold text-foreground">
          {section.title}
        </span>
        {section.topicSlug && (
          <Badge variant="warning" className="text-[10px]">{section.topicSlug}</Badge>
        )}
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
      <DropdownMenuTrigger render={<Button variant="outline" className="w-full py-3 h-auto border-2 border-dashed text-sm font-medium text-muted-foreground" />}>
        <Plus className="size-3.5" />
        Add Row
      </DropdownMenuTrigger>
      <DropdownMenuContent align="center" className="w-64">
        {templates.map((t) => (
          <DropdownMenuItem key={t.slug} onClick={() => onAdd(t.slug)}>
            <div>
              <div className="font-medium">{t.displayName}</div>
              {t.description && (
                <div className="text-xs text-muted-foreground mt-0.5">
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

function PostTypeBadge({ type }: { type: string | null | undefined }) {
  if (!type) return null;
  const colors: Record<string, string> = {
    story: "#2563EB",
    notice: "#D97706",
    exchange: "#7C3AED",
    event: "#DB2777",
    spotlight: "#059669",
    reference: "#6B7280",
  };
  return <Badge color={colors[type]} className="text-[10px] h-4">{type}</Badge>;
}

function WeightBadge({ weight }: { weight: string | null | undefined }) {
  if (!weight) return null;
  const variantMap: Record<string, "default" | "secondary" | "outline"> = {
    heavy: "default",
    medium: "secondary",
    light: "outline",
  };
  return (
    <Badge variant={variantMap[weight] || "secondary"} className="text-[10px] h-4">
      {weight}
    </Badge>
  );
}

function PostStatusBadge({ status }: { status: string }) {
  const variantMap: Record<string, "success" | "info" | "danger" | "secondary"> = {
    active: "success",
    draft: "info",
    rejected: "danger",
  };
  return (
    <Badge variant={variantMap[status] || "secondary"} className="text-[10px] h-4">
      {status}
    </Badge>
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
