"use client";

import { useState, useCallback, useMemo, useEffect } from "react";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { MediaCard } from "@/components/admin/MediaCard";
import { MediaUploadZone } from "@/components/admin/MediaUploadZone";
import {
  MediaFilters,
  type MediaFilterState,
} from "@/components/admin/MediaFilters";
import { MediaDetailPanel } from "@/components/admin/MediaDetailPanel";
import { PaginationControls } from "@/components/ui/PaginationControls";
import { Button } from "@/components/ui/button";
import { Trash2 } from "lucide-react";
import { useMediaUpload } from "@/lib/hooks/useMediaUpload";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { MediaLibraryQuery, DeleteMediaMutation } from "@/lib/graphql/media";

const PAGE_SIZE = 24;

// ---------------------------------------------------------------------------
// Search debounce — avoid refetching on every keystroke.
// ---------------------------------------------------------------------------
function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}

export default function MediaLibraryPage() {
  const pagination = useOffsetPagination({ pageSize: PAGE_SIZE });
  const [filters, setFilters] = useState<MediaFilterState>({
    search: "",
    unusedOnly: false,
    sort: "newest",
  });
  const debouncedSearch = useDebounced(filters.search, 200);

  // When filters change, reset to page 0.
  useEffect(() => {
    pagination.reset();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debouncedSearch, filters.unusedOnly]);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [bulkSelected, setBulkSelected] = useState<Set<string>>(new Set());
  const { uploads, uploadFiles, clearUploads } = useMediaUpload();

  const [{ data, fetching, error }, reexecute] = useQuery({
    query: MediaLibraryQuery,
    variables: {
      limit: pagination.variables.first,
      offset: pagination.variables.offset,
      search: debouncedSearch || undefined,
      unusedOnly: filters.unusedOnly,
    },
    requestPolicy: "cache-and-network",
  });

  const [, deleteMedia] = useMutation(DeleteMediaMutation);

  const rawMedia = data?.mediaLibrary?.media ?? [];
  const totalCount = data?.mediaLibrary?.totalCount ?? 0;
  const hasNextPage = data?.mediaLibrary?.hasNextPage ?? false;

  // Client-side sort within the current page. Newest is server-default;
  // other sorts reorder whatever the server returned. Not ideal for
  // cross-page ordering but meaningful within a page and keeps the server
  // query simple.
  const media = useMemo(() => {
    const arr = [...rawMedia];
    switch (filters.sort) {
      case "oldest":
        return arr.reverse();
      case "most_used":
        return arr.sort((a, b) => (b.usageCount ?? 0) - (a.usageCount ?? 0));
      case "largest":
        return arr.sort((a, b) => b.sizeBytes - a.sizeBytes);
      default:
        return arr;
    }
  }, [rawMedia, filters.sort]);

  const selectedMedia = useMemo(
    () => media.find((m) => m.id === selectedId) ?? null,
    [media, selectedId],
  );

  const handleFilesDropped = useCallback(
    async (files: File[]) => {
      await uploadFiles(files);
      reexecute({ requestPolicy: "network-only" });
    },
    [uploadFiles, reexecute],
  );

  const handleSelectCard = useCallback((id: string) => {
    setSelectedId(id);
    setDetailOpen(true);
  }, []);

  const handleDeleted = useCallback(
    (id: string) => {
      setBulkSelected((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
      if (selectedId === id) setSelectedId(null);
      reexecute({ requestPolicy: "network-only" });
    },
    [selectedId, reexecute],
  );

  const handleBulkToggle = useCallback((id: string, checked: boolean) => {
    setBulkSelected((prev) => {
      const next = new Set(prev);
      if (checked) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);

  const handleBulkDelete = useCallback(async () => {
    const count = bulkSelected.size;
    if (count === 0) return;
    if (!confirm(`Delete ${count} item${count === 1 ? "" : "s"}? This cannot be undone.`)) return;
    for (const id of bulkSelected) {
      await deleteMedia({ id }, { additionalTypenames: ["Media", "MediaConnection"] });
    }
    setBulkSelected(new Set());
    if (selectedId && bulkSelected.has(selectedId)) setSelectedId(null);
    reexecute({ requestPolicy: "network-only" });
  }, [bulkSelected, deleteMedia, selectedId, reexecute]);

  const activeUploads = uploads.filter(
    (u) => u.progress !== "done" && u.progress !== "error",
  );

  return (
    <div className="p-6 min-w-0">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-2xl font-semibold text-text-primary">Media Library</h1>
          <p className="text-sm text-text-muted mt-1">
            {totalCount} item{totalCount !== 1 ? "s" : ""}
            {filters.unusedOnly && " (unused only)"}
            {debouncedSearch && ` matching "${debouncedSearch}"`}
          </p>
        </div>

        {bulkSelected.size > 0 && (
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-muted">
              {bulkSelected.size} selected
            </span>
            <Button variant="outline" size="sm" onClick={() => setBulkSelected(new Set())}>
              Clear
            </Button>
            <Button variant="destructive" size="sm" onClick={handleBulkDelete}>
              <Trash2 className="size-3.5 mr-1.5" />
              Delete selected
            </Button>
          </div>
        )}
      </div>

      {/* Filters */}
      <div className="mb-4">
        <MediaFilters value={filters} onChange={setFilters} />
      </div>

      {/* Upload Zone */}
      <MediaUploadZone
        onFiles={handleFilesDropped}
        disabled={activeUploads.length > 0}
        className="mb-4"
      />

      {/* Upload Progress */}
      {uploads.length > 0 && (
        <div className="mb-4 space-y-2">
          {uploads.map((u, i) => (
            <div
              key={i}
              className="flex items-center gap-3 px-4 py-2 bg-card rounded-lg border border-border"
            >
              <span className="text-sm text-text-body truncate flex-1">
                {u.file.name}
              </span>
              {u.progress === "done" ? (
                <span className="text-xs font-medium text-success-text bg-success-bg px-2 py-0.5 rounded-full">
                  Done
                </span>
              ) : u.progress === "error" ? (
                <span className="text-xs font-medium text-danger-text bg-danger-bg px-2 py-0.5 rounded-full">
                  {u.error || "Failed"}
                </span>
              ) : (
                <span className="text-xs font-medium text-info-text bg-info-bg px-2 py-0.5 rounded-full">
                  {u.progress === "requesting"
                    ? "Preparing..."
                    : u.progress === "uploading"
                      ? "Uploading..."
                      : "Saving..."}
                </span>
              )}
            </div>
          ))}
          {uploads.every(
            (u) => u.progress === "done" || u.progress === "error",
          ) && (
            <Button variant="ghost" size="sm" onClick={clearUploads}>
              Clear upload history
            </Button>
          )}
        </div>
      )}

      {/* Grid / empty / error states */}
      {fetching && media.length === 0 ? (
        <AdminLoader label="Loading media..." />
      ) : error ? (
        <div className="text-center py-12">
          <p className="text-sm text-danger-text">{error.message}</p>
        </div>
      ) : media.length === 0 ? (
        <div className="text-center py-16">
          <p className="text-text-muted">
            {debouncedSearch
              ? `No media matches "${debouncedSearch}".`
              : filters.unusedOnly
                ? "Everything in the library is in use. "
                : "No media uploaded yet. Drop files above to get started."}
          </p>
        </div>
      ) : (
        <>
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
            {media.map((item) => (
              <MediaCard
                key={item.id}
                id={item.id}
                filename={item.filename}
                url={item.url}
                contentType={item.contentType}
                sizeBytes={item.sizeBytes}
                altText={item.altText}
                width={item.width}
                height={item.height}
                usageCount={item.usageCount}
                selected={selectedId === item.id}
                onSelect={handleSelectCard}
                bulkChecked={bulkSelected.has(item.id)}
                onBulkToggle={handleBulkToggle}
              />
            ))}
          </div>

          {(pagination.currentPage > 0 || hasNextPage) && (
            <div className="mt-8">
              <PaginationControls
                pageInfo={pagination.buildPageInfo(hasNextPage)}
                totalCount={totalCount}
                currentPage={pagination.currentPage}
                pageSize={pagination.pageSize}
                onNextPage={pagination.goToNextPage}
                onPreviousPage={pagination.goToPreviousPage}
                loading={fetching}
              />
            </div>
          )}
        </>
      )}

      {/* Detail panel */}
      <MediaDetailPanel
        media={selectedMedia}
        open={detailOpen && !!selectedMedia}
        onOpenChange={(open) => {
          setDetailOpen(open);
          if (!open) setSelectedId(null);
        }}
        onDeleted={handleDeleted}
      />
    </div>
  );
}
