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
  Pencil,
  Trash2,
  Plus,
  GripVertical,
  ExternalLink,
  Lock,
  ChevronRight,
  Grid2x2Plus,
  ListStart,
  LayoutDashboard,
  FilePenLine,
} from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
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
  AddWidgetMutation,
  UpdateWidgetMutation,
  RemoveWidgetMutation,
  AddSectionMutation,
  UpdateSectionMutation,
  DeleteSectionMutation,
  AssignRowToSectionMutation,
  ReorderSectionsMutation,
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
type EditionWidget = Edition["widgets"][number];
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

const WIDGET_TYPES = [
  { type: "section_header", label: "Section Header", description: "Full-width divider with heading", defaultConfig: { title: "Section Title" } },
  { type: "weather", label: "Weather", description: "County weather forecast card", defaultConfig: {} },
  { type: "hotline_bar", label: "Hotline Bar", description: "Phone numbers and resources", defaultConfig: { lines: [{ label: "Crisis Line", phone: "988" }] } },
  { type: "section_sep", label: "Section Separator", description: "Visual divider between sections", defaultConfig: {} },
] as const;

type WidgetTypeOption = (typeof WIDGET_TYPES)[number];

function WidgetPickerDialog({
  open,
  onOpenChange,
  onSelect,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (wt: WidgetTypeOption) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Choose widget type</DialogTitle>
          <DialogDescription>
            Select a widget to insert into the layout.
          </DialogDescription>
        </DialogHeader>
        <div className="grid grid-cols-2 gap-3 py-2">
          {WIDGET_TYPES.map((wt) => (
            <button
              key={wt.type}
              onClick={() => onSelect(wt)}
              className="flex flex-col items-start gap-1 rounded-lg border-2 border-border p-3 text-left transition-colors hover:bg-muted/50 hover:border-muted-foreground/30"
            >
              <span className="text-sm font-semibold text-foreground">
                {wt.label}
              </span>
              <span className="text-[11px] text-muted-foreground leading-tight">
                {wt.description}
              </span>
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
  currentSlug: string;
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
              <EditionWidgetsView edition={edition} refetchEdition={refetchEdition} />
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
  const [, addWidgetMut] = useMutation(AddWidgetMutation);
  const [, removeWidgetMut] = useMutation(RemoveWidgetMutation);
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
  const rowIds = useMemo(() => sortedRows.map((r) => r.id), [sortedRows]);
  const sectionIds = useMemo(() => sections.map((s) => `section-${s.id}`), [sections]);

  // DnD
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } })
  );

  const [activeRowId, setActiveRowId] = useState<string | null>(null);
  const [activeSectionId, setActiveSectionId] = useState<string | null>(null);

  const handleDragStart = useCallback((event: DragStartEvent) => {
    const dragType = event.active.data.current?.type;
    if (dragType === "row") {
      setActiveRowId(event.active.id as string);
    } else if (dragType === "section") {
      setActiveSectionId(event.active.id as string);
    } else {
      setActiveSlotId(event.active.id as string);
    }
  }, []);

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const dragType = event.active.data.current?.type;
      const { active, over } = event;

      // Section drag
      if (dragType === "section") {
        setActiveSectionId(null);
        if (!over || !edition || active.id === over.id) return;
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
      if (dragType === "row") {
        setActiveRowId(null);
        if (!over || !edition || active.id === over.id) return;
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

  const handleAddWidget = useCallback(
    async (widgetType: string, sortOrder: number, sectionId: string | null, config: Record<string, unknown>) => {
      await addWidgetMut(
        { editionId: edition!.id, widgetType, sortOrder, sectionId, config: JSON.stringify(config) },
        mutCtx
      );
      refetchEdition({ requestPolicy: "network-only" });
    },
    [edition, addWidgetMut, mutCtx, refetchEdition]
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
                <TabsTrigger value="arrange"><ListStart className="size-3.5 mr-1.5" />Arrange</TabsTrigger>
                <TabsTrigger value="edit"><LayoutDashboard className="size-3.5 mr-1.5" />Edit</TabsTrigger>
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
        <SortableContext items={[...rowIds, ...sectionIds]} strategy={verticalListSortingStrategy}>
          {sortedRows.length === 0 ? (
            <div className="text-muted-foreground text-center py-12 bg-card rounded-lg border border-border">
              <p className="text-lg mb-2">Empty broadsheet</p>
              <p className="text-sm">
                Click &ldquo;Regenerate Layout&rdquo; to auto-populate, or add rows manually.
              </p>
            </div>
          ) : (
            <SectionGroupedLayout
              rows={sortedRows}
              sections={sections}
              widgets={edition.widgets ?? []}
              isEditable={isEditable}
              isDragging={isDragging}
              allRowsCollapsed={allRowsCollapsed}
              rowTemplates={rowTemplates}
              postTemplates={postTemplates}
              onMoveRow={handleMoveRow}
              onDeleteRow={handleDeleteRow}
              onChangeRowTemplate={handleChangeRowTemplate}
              onChangeTemplate={handleChangeTemplate}
              onRemovePost={handleRemovePost}
              onViewPost={(postId) => router.push(`/admin/posts/${postId}`)}
              onAddWidget={handleAddWidget}
              onRemoveWidget={handleRemoveWidget}
              onAddRow={handleAddRow}
              onAddSection={handleAddSection}
              onUpdateSection={handleUpdateSection}
              onDeleteSection={handleDeleteSection}
            />
          )}
        </SortableContext>

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
      <p className="text-sm text-muted-foreground mb-4">
        {posts.length} post{posts.length !== 1 ? "s" : ""} placed in this edition.
      </p>

      {posts.length === 0 ? (
        <div className="text-muted-foreground text-center py-12 text-sm">
          No posts placed in this edition yet.
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {posts.map((post) => (
            <div
              key={post.id}
              onClick={() => router.push(`/admin/posts/${post.id}`)}
              className="bg-card rounded-lg border border-border p-4 hover:shadow-md transition-shadow cursor-pointer"
            >
              <div className="text-sm font-medium text-foreground mb-2">
                {post.title}
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <PostTypeBadge type={post.postType} />
                {post.weight && <WeightBadge weight={post.weight} />}
                <span className="text-xs text-muted-foreground">
                  {post.rowTemplate}
                </span>
                <PostStatusBadge status={post.status} />
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

// ─── Edition Widgets View ───────────────────────────────────────────────────

function EditionWidgetsView({
  edition,
  refetchEdition,
}: {
  edition: Edition;
  refetchEdition: (opts?: any) => void;
}) {
  const [, updateWidgetMut] = useMutation(UpdateWidgetMutation);
  const [, removeWidgetMut] = useMutation(RemoveWidgetMutation);
  const mutCtx = useMemo(
    () => ({ additionalTypenames: ["Edition", "EditionWidget"] }),
    []
  );

  const [editingWidget, setEditingWidget] = useState<string | null>(null);
  const [editConfig, setEditConfig] = useState("");

  const allWidgets = useMemo(
    () => [...(edition.widgets ?? [])].sort((a, b) => a.sortOrder - b.sortOrder),
    [edition]
  );

  const isEditable = edition.status !== "published" && edition.status !== "archived";

  const handleUpdate = async (widgetId: string) => {
    try {
      JSON.parse(editConfig);
    } catch {
      alert("Invalid JSON config");
      return;
    }
    await updateWidgetMut({ id: widgetId, config: editConfig }, mutCtx);
    refetchEdition({ requestPolicy: "network-only" });
    setEditingWidget(null);
  };

  const handleRemove = async (widgetId: string) => {
    if (!confirm("Remove this widget?")) return;
    await removeWidgetMut({ id: widgetId }, mutCtx);
    refetchEdition({ requestPolicy: "network-only" });
  };

  return (
    <>
      <div className="flex items-center justify-between mb-4">
        <p className="text-sm text-muted-foreground">
          {allWidgets.length} widget{allWidgets.length !== 1 ? "s" : ""} in this edition.
          Add widgets from the Broadsheet tab using the inline inserters.
        </p>
      </div>

      {allWidgets.length === 0 ? (
        <div className="text-muted-foreground text-center py-12 text-sm">
          No widgets in this edition yet.
        </div>
      ) : (
        <div className="space-y-3">
          {allWidgets.map((widget) => (
            <div
              key={widget.id}
              className="bg-card rounded-lg border border-border p-4"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <Badge variant="spotlight">{widget.widgetType}</Badge>
                    <span className="text-xs text-muted-foreground">
                      Sort order: {widget.sortOrder}
                      {widget.sectionId && ` · Section`}
                    </span>
                  </div>

                  {editingWidget === widget.id ? (
                    <div className="mt-2">
                      <Textarea
                        value={editConfig}
                        onChange={(e) => setEditConfig(e.target.value)}
                        className="font-mono h-24 min-h-0 resize-none"
                      />
                      <div className="flex gap-2 mt-2">
                        <Button variant="admin" size="sm" onClick={() => handleUpdate(widget.id)}>
                          Save
                        </Button>
                        <Button variant="ghost" size="sm" onClick={() => setEditingWidget(null)}>
                          Cancel
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <pre className="mt-1 text-xs text-muted-foreground bg-muted rounded p-2 overflow-x-auto max-w-lg">
                      {(() => {
                        try {
                          return JSON.stringify(JSON.parse(widget.config), null, 2);
                        } catch {
                          return widget.config;
                        }
                      })()}
                    </pre>
                  )}
                </div>

                {isEditable && editingWidget !== widget.id && (
                  <div className="flex gap-1 ml-3 shrink-0">
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      onClick={() => {
                        setEditingWidget(widget.id);
                        setEditConfig(widget.config);
                      }}
                      title="Edit config"
                    >
                      <Pencil className="size-3.5" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      onClick={() => handleRemove(widget.id)}
                      className="hover:text-destructive"
                      title="Remove widget"
                    >
                      <Trash2 className="size-3.5" />
                    </Button>
                  </div>
                )}
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
  isDragging,
  collapsed,
  rowTemplates,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeRowTemplate,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
}: {
  row: EditionRow;
  rowIndex: number;
  totalRows: number;
  isEditable: boolean;
  isDragging: boolean;
  collapsed: boolean;
  rowTemplates: RowTemplate[];
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeRowTemplate: (rowId: string, slug: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
}) {
  const {
    attributes: sortableAttributes,
    listeners: sortableListeners,
    setNodeRef: setSortableRef,
    transform: sortableTransform,
    transition: sortableTransition,
    isDragging: isSortableDragging,
  } = useSortable({ id: row.id, data: { type: "row" }, disabled: !isEditable || !collapsed });

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

  const postCount = row.slots.length;

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
          <span className="text-sm font-semibold text-foreground">
            {row.rowTemplate.displayName}
          </span>
          {isEditable && rowTemplates.length > 0 && (
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={() => setTemplatePickerOpen(true)}
              className="text-muted-foreground hover:text-foreground"
              title="Change row template"
            >
              <Grid2x2Plus className="size-3" />
            </Button>
          )}
        </div>
        <div className="flex items-center gap-1">
          {collapsed && (
            <span className="text-xs text-muted-foreground">
              {postCount} post{postCount !== 1 ? "s" : ""}
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
        <div className="p-4">
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
              />
            ))}
          </div>
        </div>
      )}
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
  const [confirmOpen, setConfirmOpen] = useState(false);

  return (
    <div className="flex items-center gap-3 rounded-lg border border-border bg-muted/30 px-3 py-2">
      <WidgetIcon type={widget.widgetType} />
      <div className="flex-1 min-w-0">
        <WidgetContent type={widget.widgetType} config={config} />
      </div>
      {isEditable && (
        <>
          <Button
            variant="ghost"
            size="xs"
            onClick={() => setConfirmOpen(true)}
            className="text-destructive hover:text-destructive shrink-0"
          >
            Remove
          </Button>
          <ConfirmDialog
            open={confirmOpen}
            onOpenChange={setConfirmOpen}
            title="Remove widget"
            description="This widget will be permanently deleted from this row."
            confirmLabel="Remove widget"
            onConfirm={() => onRemove(widget.id)}
          />
        </>
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
  const icon = icons[type] ?? { bg: "bg-muted text-muted-foreground", label: "?" };
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
          <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Section Header</div>
          <div className="text-sm font-semibold text-foreground truncate">
            {(config.title as string) || "Untitled"}
          </div>
          {typeof config.subtitle === "string" && config.subtitle && (
            <div className="text-xs text-muted-foreground truncate">{config.subtitle}</div>
          )}
        </div>
      );
    case "weather":
      return (
        <div>
          <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Weather</div>
          <div className="text-sm text-foreground">
            {config.location_id ? `Location: ${config.location_id}` : "County default"}
          </div>
        </div>
      );
    case "hotline_bar": {
      const lines = Array.isArray(config.lines) ? config.lines : [];
      return (
        <div>
          <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Hotline Bar</div>
          <div className="text-sm text-foreground">
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
          <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide">{type}</div>
          <div className="text-xs text-muted-foreground">Unknown widget type</div>
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

type LayoutItem =
  | { type: "row"; data: EditionRow; sortOrder: number }
  | { type: "widget"; data: EditionWidget; sortOrder: number };

function SectionGroupedLayout({
  rows,
  sections,
  widgets,
  isEditable,
  isDragging,
  allRowsCollapsed,
  rowTemplates,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeRowTemplate,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddWidget,
  onRemoveWidget,
  onAddRow,
  onAddSection,
  onUpdateSection,
  onDeleteSection,
}: {
  rows: EditionRow[];
  sections: EditionSection[];
  widgets: EditionWidget[];
  isEditable: boolean;
  isDragging: boolean;
  allRowsCollapsed: boolean;
  rowTemplates: RowTemplate[];
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeRowTemplate: (rowId: string, slug: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddWidget: (widgetType: string, sortOrder: number, sectionId: string | null, config: Record<string, unknown>) => void;
  onRemoveWidget: (widgetId: string) => void;
  onAddRow: (templateSlug: string, sortOrder?: number) => void;
  onAddSection: (title: string, sortOrder?: number) => void;
  onUpdateSection: (sectionId: string, title: string) => void;
  onDeleteSection: (sectionId: string) => void;
}) {
  // Build unified layout items per section (and ungrouped)
  const buildItems = useCallback(
    (sectionId: string | null): LayoutItem[] => {
      const items: LayoutItem[] = [
        ...rows
          .filter((r) => (sectionId ? r.sectionId === sectionId : !r.sectionId))
          .map((r) => ({ type: "row" as const, data: r, sortOrder: r.sortOrder })),
        ...widgets
          .filter((w) => (sectionId ? w.sectionId === sectionId : !w.sectionId))
          .map((w) => ({ type: "widget" as const, data: w, sortOrder: w.sortOrder })),
      ];
      return items.sort((a, b) => a.sortOrder - b.sortOrder);
    },
    [rows, widgets]
  );

  const ungroupedItems = useMemo(() => buildItems(null), [buildItems]);

  // Calculate sort order for inserting between items
  const getInsertSortOrder = (items: LayoutItem[], index: number) => {
    if (items.length === 0) return 0;
    if (index <= 0) return (items[0]?.sortOrder ?? 0) - 10;
    if (index >= items.length) return (items[items.length - 1]?.sortOrder ?? 0) + 10;
    const prev = items[index - 1].sortOrder;
    const next = items[index].sortOrder;
    return Math.floor((prev + next) / 2);
  };

  const renderLayoutItems = (items: LayoutItem[], sectionId: string | null) => (
    <div className="space-y-3">
      {items.map((item, idx) => (
        <div key={item.type === "row" ? item.data.id : item.data.id}>
          {isEditable && allRowsCollapsed && (
            <InlineInserter
              sortOrder={getInsertSortOrder(items, idx)}
              sectionId={sectionId}
              rowTemplates={rowTemplates}
              onAddRow={onAddRow}
              onAddWidget={onAddWidget}
            />
          )}
          {item.type === "row" ? (
            <RowEditor
              row={item.data}
              rowIndex={idx}
              totalRows={rows.length}
              isEditable={isEditable}
              isDragging={isDragging}
              collapsed={allRowsCollapsed}
              rowTemplates={rowTemplates}
              postTemplates={postTemplates}
              onMoveRow={onMoveRow}
              onDeleteRow={onDeleteRow}
              onChangeRowTemplate={onChangeRowTemplate}
              onChangeTemplate={onChangeTemplate}
              onRemovePost={onRemovePost}
              onViewPost={onViewPost}
            />
          ) : (
            <WidgetItem
              widget={item.data}
              isEditable={isEditable}
              collapsed={allRowsCollapsed}
              onRemove={onRemoveWidget}
            />
          )}
        </div>
      ))}
      {/* Trailing inserter after last item */}
      {isEditable && allRowsCollapsed && (
        <InlineInserter
          sortOrder={getInsertSortOrder(items, items.length)}
          sectionId={sectionId}
          rowTemplates={rowTemplates}
          onAddRow={onAddRow}
          onAddWidget={onAddWidget}
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
          {renderLayoutItems(ungroupedItems, null)}
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
              allRowsCollapsed={allRowsCollapsed}
              rowTemplates={rowTemplates}
              postTemplates={postTemplates}
              onMoveRow={onMoveRow}
              onDeleteRow={onDeleteRow}
              onChangeRowTemplate={onChangeRowTemplate}
              onChangeTemplate={onChangeTemplate}
              onRemovePost={onRemovePost}
              onViewPost={onViewPost}
              onAddWidget={onAddWidget}
              onRemoveWidget={onRemoveWidget}
              onAddRow={onAddRow}
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

      {/* If no sections yet and no ungrouped items, show initial inserters */}
      {isEditable && allRowsCollapsed && sections.length === 0 && ungroupedItems.length === 0 && (
        <div className="space-y-2">
          <InlineInserter
            sortOrder={0}
            sectionId={null}
            rowTemplates={rowTemplates}
            onAddRow={onAddRow}
            onAddWidget={onAddWidget}
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
  allRowsCollapsed,
  rowTemplates,
  postTemplates,
  onMoveRow,
  onDeleteRow,
  onChangeRowTemplate,
  onChangeTemplate,
  onRemovePost,
  onViewPost,
  onAddWidget,
  onRemoveWidget,
  onAddRow,
  onUpdateSection,
  onDeleteSection,
}: {
  section: EditionSection;
  items: LayoutItem[];
  rows: EditionRow[];
  isEditable: boolean;
  isDragging: boolean;
  allRowsCollapsed: boolean;
  rowTemplates: RowTemplate[];
  postTemplates: PostTemplate[];
  onMoveRow: (rowId: string, dir: "up" | "down") => void;
  onDeleteRow: (rowId: string) => void;
  onChangeRowTemplate: (rowId: string, slug: string) => void;
  onChangeTemplate: (slotId: string, template: string) => void;
  onRemovePost: (slotId: string) => void;
  onViewPost: (postId: string) => void;
  onAddWidget: (widgetType: string, sortOrder: number, sectionId: string | null, config: Record<string, unknown>) => void;
  onRemoveWidget: (widgetId: string) => void;
  onAddRow: (templateSlug: string, sortOrder?: number) => void;
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
  } = useSortable({ id: sortableId, data: { type: "section" }, disabled: !isEditable || !allRowsCollapsed });

  const sortableStyle = {
    transform: CSS.Transform.toString(sortableTransform),
    transition: sortableTransition,
  };

  const handleSave = () => {
    onUpdateSection(section.id, editTitle);
    setIsEditing(false);
  };

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
            <div className="space-y-3">
              {items.map((item, idx) => (
                <div key={item.type === "row" ? item.data.id : item.data.id}>
                  {isEditable && allRowsCollapsed && (
                    <InlineInserter
                      sortOrder={getInsertSortOrder(idx)}
                      sectionId={section.id}
                      rowTemplates={rowTemplates}
                      onAddRow={onAddRow}
                      onAddWidget={onAddWidget}
                    />
                  )}
                  {item.type === "row" ? (
                    <RowEditor
                      row={item.data}
                      rowIndex={idx}
                      totalRows={rows.length}
                      isEditable={isEditable}
                      isDragging={isDragging}
                      collapsed={allRowsCollapsed}
                      rowTemplates={rowTemplates}
                      postTemplates={postTemplates}
                      onMoveRow={onMoveRow}
                      onDeleteRow={onDeleteRow}
                      onChangeRowTemplate={onChangeRowTemplate}
                      onChangeTemplate={onChangeTemplate}
                      onRemovePost={onRemovePost}
                      onViewPost={onViewPost}
                    />
                  ) : (
                    <WidgetItem
                      widget={item.data}
                      isEditable={isEditable}
                      collapsed={allRowsCollapsed}
                      onRemove={onRemoveWidget}
                    />
                  )}
                </div>
              ))}
              {isEditable && allRowsCollapsed && (
                <InlineInserter
                  sortOrder={getInsertSortOrder(items.length)}
                  sectionId={section.id}
                  rowTemplates={rowTemplates}
                  onAddRow={onAddRow}
                  onAddWidget={onAddWidget}
                />
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ─── WidgetItem (compact bar for arrange mode, full card for edit mode) ──────

function WidgetItem({
  widget,
  isEditable,
  collapsed,
  onRemove,
}: {
  widget: EditionWidget;
  isEditable: boolean;
  collapsed: boolean;
  onRemove: (id: string) => void;
}) {
  const config = parseWidgetConfig(widget.config);
  const [confirmOpen, setConfirmOpen] = useState(false);

  if (collapsed) {
    // Compact bar for arrange mode
    return (
      <div className="flex items-center gap-3 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2">
        <WidgetIcon type={widget.widgetType} />
        <span className="text-sm font-medium text-foreground truncate flex-1">
          {widget.widgetType.replace(/_/g, " ")}
        </span>
        {isEditable && (
          <>
            <Button
              variant="ghost"
              size="icon-xs"
              onClick={() => setConfirmOpen(true)}
              className="hover:text-destructive shrink-0"
            >
              <X className="size-3.5" />
            </Button>
            <ConfirmDialog
              open={confirmOpen}
              onOpenChange={setConfirmOpen}
              title="Remove widget"
              description="This widget will be permanently deleted from this edition."
              confirmLabel="Remove widget"
              onConfirm={() => onRemove(widget.id)}
            />
          </>
        )}
      </div>
    );
  }

  // Full card for edit mode
  return (
    <WidgetCard widget={widget} isEditable={isEditable} onRemove={onRemove} />
  );
}

// ─── InlineInserter (thin row between layout items) ─────────────────────────

function InlineInserter({
  sortOrder,
  sectionId,
  rowTemplates,
  onAddRow,
  onAddWidget,
}: {
  sortOrder: number;
  sectionId: string | null;
  rowTemplates: RowTemplate[];
  onAddRow: (templateSlug: string, sortOrder?: number) => void;
  onAddWidget: (widgetType: string, sortOrder: number, sectionId: string | null, config: Record<string, unknown>) => void;
}) {
  const [templatePickerOpen, setTemplatePickerOpen] = useState(false);
  const [widgetPickerOpen, setWidgetPickerOpen] = useState(false);

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
        <Button
          variant="ghost"
          size="xs"
          className="text-[10px] text-muted-foreground h-5 px-1.5 bg-background"
          onClick={() => setWidgetPickerOpen(true)}
        >
          <Plus className="size-3" />
          Widget
        </Button>
      </div>

      <WidgetPickerDialog
        open={widgetPickerOpen}
        onOpenChange={setWidgetPickerOpen}
        onSelect={(wt) => {
          onAddWidget(wt.type, sortOrder, sectionId, wt.defaultConfig);
          setWidgetPickerOpen(false);
        }}
      />

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
          : isDragging && hasRoom && isEditable
            ? "ring-1 ring-amber-200 bg-amber-50/30"
            : hasRoom && isEditable
              ? "bg-muted/30"
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
      {hasRoom && (
        <div
          className={`rounded-lg border-2 border-dashed p-3 flex items-center justify-center gap-2 ${
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
              <WeightBadge weight={templateSlot.weight} />
              <span className="text-xs text-muted-foreground">
                {templateSlot.count - editionSlots.length} open
              </span>
            </>
          )}
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
          <div className="text-sm font-medium text-foreground truncate">
            {slot.post.title}
          </div>
          <div className="flex flex-wrap items-center gap-1.5 mt-1.5">
            <PostTypeBadge type={slot.post.postType} />
            <WeightBadge weight={slot.post.weight} />
            {isEditable && postTemplates.length > 0 && (
              <Button
                variant="ghost"
                size="xs"
                className="h-5 text-[10px] px-1.5 border border-border text-muted-foreground"
                onClick={() => setTemplatePickerOpen(true)}
              >
                {postTemplates.find((pt) => pt.slug === slot.postTemplate)?.displayName ?? slot.postTemplate}
              </Button>
            )}
            {!isEditable && (
              <span className="text-[10px] text-muted-foreground">{slot.postTemplate}</span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-0.5 shrink-0">
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={() => onViewPost(slot.post.id)}
            className="text-muted-foreground hover:text-amber-600"
            title="Edit post"
          >
            <FilePenLine className="size-3.5" />
          </Button>
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
        description={`Remove "${slot.post.title}" from this slot? The post returns to the unassigned pool and can be placed in another slot. It is not deleted.`}
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
  return (
    <div className="rounded-lg border border-amber-300 bg-card shadow-xl p-3 max-w-xs rotate-1 scale-[1.02]">
      <div className="text-sm font-medium text-foreground truncate">
        {slot.post.title}
      </div>
      <div className="flex items-center gap-1.5 mt-1">
        <PostTypeBadge type={slot.post.postType} />
        <WeightBadge weight={slot.post.weight} />
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
