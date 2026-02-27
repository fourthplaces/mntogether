"use client";

import { useState, useCallback } from "react";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { MediaCard } from "@/components/admin/MediaCard";
import { MediaUploadZone } from "@/components/admin/MediaUploadZone";
import { useMediaUpload } from "@/lib/hooks/useMediaUpload";
import { MediaLibraryQuery, DeleteMediaMutation } from "@/lib/graphql/media";

const PAGE_SIZE = 24;

export default function MediaLibraryPage() {
  const [offset, setOffset] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const { uploads, uploadFiles, clearUploads } = useMediaUpload();

  const [{ data, fetching, error }, reexecute] = useQuery({
    query: MediaLibraryQuery,
    variables: { limit: PAGE_SIZE, offset },
  });

  const [, deleteMedia] = useMutation(DeleteMediaMutation);

  const media = data?.mediaLibrary?.media ?? [];
  const totalCount = data?.mediaLibrary?.totalCount ?? 0;
  const hasNextPage = data?.mediaLibrary?.hasNextPage ?? false;

  const handleFilesDropped = useCallback(
    async (files: File[]) => {
      await uploadFiles(files);
      // Refresh the grid after uploads complete
      reexecute({ requestPolicy: "network-only" });
    },
    [uploadFiles, reexecute]
  );

  const handleDelete = useCallback(
    async (id: string) => {
      if (!confirm("Delete this media item? This cannot be undone.")) return;
      await deleteMedia(
        { id },
        { additionalTypenames: ["Media", "MediaConnection"] }
      );
      if (selectedId === id) setSelectedId(null);
      reexecute({ requestPolicy: "network-only" });
    },
    [deleteMedia, selectedId, reexecute]
  );

  const handleCopyUrl = useCallback(() => {
    const item = media.find((m) => m.id === selectedId);
    if (item) {
      navigator.clipboard.writeText(item.url);
    }
  }, [media, selectedId]);

  const activeUploads = uploads.filter(
    (u) => u.progress !== "done" && u.progress !== "error"
  );

  return (
    <div className="p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-semibold text-text-primary">
            Media Library
          </h1>
          <p className="text-sm text-text-muted mt-1">
            {totalCount} item{totalCount !== 1 ? "s" : ""}
          </p>
        </div>

        {selectedId && (
          <button
            onClick={handleCopyUrl}
            className="px-4 py-2 text-sm font-medium text-admin-accent border border-admin-accent rounded-lg hover:bg-admin-accent/5 transition-colors"
          >
            Copy URL
          </button>
        )}
      </div>

      {/* Upload Zone */}
      <MediaUploadZone
        onFiles={handleFilesDropped}
        disabled={activeUploads.length > 0}
        className="mb-6"
      />

      {/* Upload Progress */}
      {uploads.length > 0 && (
        <div className="mb-6 space-y-2">
          {uploads.map((u, i) => (
            <div
              key={i}
              className="flex items-center gap-3 px-4 py-2 bg-surface-subtle rounded-lg border border-border"
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
            (u) => u.progress === "done" || u.progress === "error"
          ) && (
            <button
              onClick={clearUploads}
              className="text-xs text-text-muted hover:text-text-secondary"
            >
              Clear upload history
            </button>
          )}
        </div>
      )}

      {/* Grid */}
      {fetching && media.length === 0 ? (
        <AdminLoader label="Loading media..." />
      ) : error ? (
        <div className="text-center py-12">
          <p className="text-sm text-danger-text">{error.message}</p>
        </div>
      ) : media.length === 0 ? (
        <div className="text-center py-16">
          <p className="text-text-muted">
            No media uploaded yet. Drop files above to get started.
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
                selected={selectedId === item.id}
                onSelect={setSelectedId}
                onDelete={handleDelete}
              />
            ))}
          </div>

          {/* Pagination */}
          {(offset > 0 || hasNextPage) && (
            <div className="flex items-center justify-center gap-4 mt-8">
              <button
                onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
                disabled={offset === 0}
                className="px-4 py-2 text-sm border border-border rounded-lg hover:bg-surface-muted disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                Previous
              </button>
              <span className="text-sm text-text-muted">
                {offset + 1}&ndash;{Math.min(offset + PAGE_SIZE, totalCount)}{" "}
                of {totalCount}
              </span>
              <button
                onClick={() => setOffset(offset + PAGE_SIZE)}
                disabled={!hasNextPage}
                className="px-4 py-2 text-sm border border-border rounded-lg hover:bg-surface-muted disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                Next
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
