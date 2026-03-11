"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
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
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { NotesListQuery, DeleteNoteMutation } from "@/lib/graphql/notes";
import { Alert } from "@/components/ui/alert";
import { Trash2, ExternalLink, AlertTriangle, Info, Bell } from "lucide-react";

// ─── Types & config ─────────────────────────────────────────────────────────

type SeverityFilter = "" | "urgent" | "notice" | "info";

const SEVERITY_TABS: { key: SeverityFilter; label: string }[] = [
  { key: "", label: "All" },
  { key: "urgent", label: "Urgent" },
  { key: "notice", label: "Notice" },
  { key: "info", label: "Info" },
];

const SEVERITY_ICON: Record<string, React.ReactNode> = {
  urgent: <AlertTriangle className="size-3.5" />,
  notice: <Bell className="size-3.5" />,
  info: <Info className="size-3.5" />,
};

const SEVERITY_BADGE_VARIANT: Record<string, "danger" | "warning" | "info"> = {
  urgent: "danger",
  notice: "warning",
  info: "info",
};

// ─── Helpers ────────────────────────────────────────────────────────────────

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 7) return `${diffDay}d ago`;
  return new Date(dateStr).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
  });
}

function truncate(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return str.slice(0, maxLen).trimEnd() + "\u2026";
}

// ─── Component ──────────────────────────────────────────────────────────────

export default function NotesPage() {
  const router = useRouter();
  const [severityFilter, setSeverityFilter] = useState<SeverityFilter>("");
  const [visibilityFilter, setVisibilityFilter] = useState<"" | "public" | "internal">("");
  const [deleteTarget, setDeleteTarget] = useState<{
    id: string;
    content: string;
  } | null>(null);

  const pagination = useOffsetPagination({ pageSize: 30 });

  // Reset pagination when filters change
  useEffect(() => {
    pagination.reset();
  }, [severityFilter, visibilityFilter]);

  // ─── Queries ──────────────────────────────────────────────────────

  const [{ data, fetching, error }] = useQuery({
    query: NotesListQuery,
    variables: {
      severity: severityFilter || null,
      isPublic: visibilityFilter === "public" ? true : visibilityFilter === "internal" ? false : null,
      limit: pagination.variables.first,
      offset: pagination.variables.offset,
    },
  });

  const [, deleteNote] = useMutation(DeleteNoteMutation);

  const notes = data?.notes?.notes || [];
  const totalCount = data?.notes?.totalCount || 0;
  const hasNextPage = totalCount > (pagination.currentPage + 1) * pagination.pageSize;
  const pageInfo = pagination.buildPageInfo(hasNextPage);

  // ─── Actions ──────────────────────────────────────────────────────

  const handleDelete = async () => {
    if (!deleteTarget) return;
    await deleteNote(
      { id: deleteTarget.id },
      { additionalTypenames: ["Note", "NoteConnection"] }
    );
    setDeleteTarget(null);
  };

  // ─── Render ───────────────────────────────────────────────────────

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-foreground">Notes</h1>
            <p className="text-muted-foreground text-sm mt-0.5">
              Alerts &amp; notices attached to organizations and posts &middot;{" "}
              {totalCount.toLocaleString()} notes
            </p>
          </div>
        </div>

        {/* Filter rows */}
        <div className="flex items-center gap-4 mb-4">
          <Tabs value={severityFilter} onValueChange={(v) => setSeverityFilter(v as SeverityFilter)}>
            <TabsList>
              {SEVERITY_TABS.map((tab) => (
                <TabsTrigger key={tab.key} value={tab.key}>
                  {tab.label}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>

          <Tabs value={visibilityFilter} onValueChange={(v) => setVisibilityFilter(v as "" | "public" | "internal")}>
            <TabsList>
              <TabsTrigger value="">All Visibility</TabsTrigger>
              <TabsTrigger value="public">Public</TabsTrigger>
              <TabsTrigger value="internal">Internal</TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        {/* Error */}
        {error && (
          <Alert variant="error" className="mb-4">
            Error: {error.message}
          </Alert>
        )}

        {/* Loading */}
        {fetching && notes.length === 0 && (
          <AdminLoader label="Loading notes..." />
        )}

        {/* Table or empty */}
        {!fetching && !error && notes.length === 0 ? (
          <div className="bg-card border border-border rounded-lg p-12 text-center">
            <h3 className="text-lg font-semibold text-foreground mb-1">
              No notes found
            </h3>
            <p className="text-muted-foreground text-sm">
              {severityFilter || visibilityFilter
                ? `No ${severityFilter || ""} ${visibilityFilter || ""} notes found.`.replace(/\s+/g, " ")
                : "No notes have been created yet."}
            </p>
          </div>
        ) : (
          notes.length > 0 && (
            <>
              <div className="rounded-lg border border-border overflow-hidden bg-card">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="pl-6">Note</TableHead>
                      <TableHead className="w-24">Severity</TableHead>
                      <TableHead className="w-24">Visibility</TableHead>
                      <TableHead className="w-40">Linked Posts</TableHead>
                      <TableHead className="w-28">Created</TableHead>
                      <TableHead className="w-14" />
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {notes.map((note) => (
                      <TableRow
                        key={note.id}
                        onClick={() => router.push(`/admin/notes/${note.id}`)}
                        className="cursor-pointer"
                      >
                        <TableCell className="pl-6">
                          <div className="text-foreground">
                            {truncate(note.content, 120)}
                          </div>
                          {note.ctaText && (
                            <div className="text-xs text-muted-foreground mt-0.5">
                              CTA: {note.ctaText}
                            </div>
                          )}
                          {note.sourceUrl && (
                            <a
                              href={note.sourceUrl}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="inline-flex items-center gap-1 text-xs text-link hover:underline mt-0.5"
                            >
                              <ExternalLink className="size-3" />
                              Source
                            </a>
                          )}
                        </TableCell>

                        <TableCell className="whitespace-nowrap">
                          <Badge
                            variant={
                              SEVERITY_BADGE_VARIANT[note.severity] || "secondary"
                            }
                          >
                            {SEVERITY_ICON[note.severity]}
                            {note.severity}
                          </Badge>
                        </TableCell>

                        <TableCell className="whitespace-nowrap">
                          <Badge
                            variant={note.isPublic ? "success" : "secondary"}
                          >
                            {note.isPublic ? "Public" : "Internal"}
                          </Badge>
                        </TableCell>

                        <TableCell>
                          {note.linkedPosts && note.linkedPosts.length > 0 ? (
                            <div className="flex flex-col gap-0.5">
                              {note.linkedPosts.slice(0, 2).map((lp) => (
                                <Link
                                  key={lp.id}
                                  href={`/admin/posts/${lp.id}`}
                                  onClick={(e) => e.stopPropagation()}
                                  className="text-xs text-link hover:underline truncate max-w-[10rem] block"
                                >
                                  {lp.title}
                                </Link>
                              ))}
                              {note.linkedPosts.length > 2 && (
                                <span className="text-xs text-muted-foreground">
                                  +{note.linkedPosts.length - 2} more
                                </span>
                              )}
                            </div>
                          ) : (
                            <span className="text-xs text-muted-foreground">
                              None
                            </span>
                          )}
                        </TableCell>

                        <TableCell className="whitespace-nowrap text-muted-foreground">
                          {timeAgo(note.createdAt)}
                        </TableCell>

                        <TableCell className="whitespace-nowrap">
                          <Button
                            variant="ghost"
                            size="icon-xs"
                            className="text-muted-foreground hover:text-destructive"
                            onClick={(e) => {
                              e.stopPropagation();
                              setDeleteTarget({
                                id: note.id,
                                content: note.content,
                              });
                            }}
                            title="Delete note"
                          >
                            <Trash2 className="size-4" />
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>

              {/* Pagination */}
              <div className="mt-4">
                <PaginationControls
                  pageInfo={pageInfo}
                  totalCount={totalCount}
                  currentPage={pagination.currentPage}
                  pageSize={pagination.pageSize}
                  onNextPage={pagination.goToNextPage}
                  onPreviousPage={pagination.goToPreviousPage}
                  loading={fetching}
                />
              </div>
            </>
          )
        )}

        {/* Delete confirmation dialog */}
        <Dialog
          open={!!deleteTarget}
          onOpenChange={(open) => !open && setDeleteTarget(null)}
        >
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Delete Note</DialogTitle>
            </DialogHeader>
            <p className="text-sm text-muted-foreground">
              This will permanently delete this note and unlink it from all
              posts and organizations.
            </p>
            {deleteTarget && (
              <div className="bg-secondary rounded-lg p-3 text-sm text-foreground mt-1">
                {truncate(deleteTarget.content, 200)}
              </div>
            )}
            <DialogFooter>
              <Button
                variant="outline"
                onClick={() => setDeleteTarget(null)}
              >
                Cancel
              </Button>
              <Button variant="destructive" onClick={handleDelete}>
                Delete
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </div>
    </div>
  );
}
