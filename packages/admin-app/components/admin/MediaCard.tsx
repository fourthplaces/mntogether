"use client";

import { X } from "lucide-react";
import { cn } from "@/lib/utils";

interface MediaCardProps {
  id: string;
  filename: string;
  url: string;
  contentType: string;
  sizeBytes: number;
  altText?: string | null;
  width?: number | null;
  height?: number | null;
  selected?: boolean;
  onSelect?: (id: string) => void;
  onDelete?: (id: string) => void;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function MediaCard({
  id,
  filename,
  url,
  contentType,
  sizeBytes,
  altText,
  selected,
  onSelect,
  onDelete,
}: MediaCardProps) {
  const isImage = contentType.startsWith("image/");

  return (
    <div
      className={cn(
        "group relative rounded-lg border-2 overflow-hidden cursor-pointer transition-all",
        selected
          ? "border-admin-accent ring-2 ring-media-selected-ring"
          : "border-border hover:border-border-strong hover:shadow-card-hover"
      )}
      onClick={() => onSelect?.(id)}
    >
      {/* Thumbnail */}
      <div className="aspect-square bg-media-checkerboard flex items-center justify-center overflow-hidden">
        {isImage ? (
          <img
            src={url}
            alt={altText || filename}
            className="w-full h-full object-cover"
            loading="lazy"
          />
        ) : (
          <div className="flex flex-col items-center gap-2 p-4 text-text-muted">
            <svg
              className="w-10 h-10"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
              />
            </svg>
            <span className="text-xs truncate max-w-full">
              {contentType.split("/")[1]?.toUpperCase() || "FILE"}
            </span>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="p-2 bg-surface-subtle">
        <p className="text-xs font-medium text-text-body truncate" title={filename}>
          {filename}
        </p>
        <p className="text-[10px] text-text-muted">
          {formatFileSize(sizeBytes)}
        </p>
      </div>

      {/* Selection check */}
      {selected && (
        <div className="absolute top-2 right-2 w-6 h-6 bg-admin-accent rounded-full flex items-center justify-center">
          <svg className="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
          </svg>
        </div>
      )}

      {/* Delete button (visible on hover) */}
      {onDelete && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onDelete(id);
          }}
          className="absolute top-2 left-2 w-6 h-6 bg-danger text-white rounded-full flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity text-xs"
          title="Delete"
        >
          <X className="w-3 h-3" />
        </button>
      )}
    </div>
  );
}
