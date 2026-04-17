"use client";

import { FileText, Check } from "lucide-react";
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
  usageCount?: number | null;
  selected?: boolean;
  onSelect?: (id: string) => void;
  /** Multi-select checkbox state (independent of the row-selection highlight). */
  bulkChecked?: boolean;
  onBulkToggle?: (id: string, checked: boolean) => void;
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
  width,
  height,
  usageCount,
  selected,
  onSelect,
  bulkChecked,
  onBulkToggle,
}: MediaCardProps) {
  const isImage = contentType.startsWith("image/");

  return (
    <div
      className={cn(
        "group relative rounded-lg border-2 overflow-hidden cursor-pointer transition-all",
        selected
          ? "border-admin-accent ring-2 ring-admin-accent/30"
          : "border-border hover:border-border-strong hover:shadow-card-hover",
      )}
      onClick={() => onSelect?.(id)}
    >
      {/* Multi-select checkbox (top-left, hover-visible unless already checked) */}
      {onBulkToggle && (
        <label
          className={cn(
            "absolute top-2 left-2 z-10 flex items-center justify-center w-5 h-5 rounded border-2 border-white bg-white/80 shadow-sm transition-opacity cursor-pointer",
            bulkChecked ? "opacity-100" : "opacity-0 group-hover:opacity-100",
          )}
          onClick={(e) => e.stopPropagation()}
        >
          <input
            type="checkbox"
            checked={!!bulkChecked}
            onChange={(e) => onBulkToggle(id, e.target.checked)}
            className="sr-only"
          />
          {bulkChecked && <Check className="w-3 h-3 text-admin-accent" />}
        </label>
      )}

      {/* Thumbnail */}
      <div className="aspect-square bg-media-checkerboard flex items-center justify-center overflow-hidden">
        {isImage ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={url}
            alt={altText || filename}
            className="w-full h-full object-cover"
            loading="lazy"
            draggable={false}
          />
        ) : (
          <div className="flex flex-col items-center gap-2 p-4 text-text-muted">
            <FileText className="w-10 h-10" />
            <span className="text-xs truncate max-w-full">
              {contentType.split("/")[1]?.toUpperCase() || "FILE"}
            </span>
          </div>
        )}
      </div>

      {/* Usage badge (bottom-right overlay) */}
      {usageCount != null && usageCount > 0 && (
        <span
          className="absolute bottom-10 right-1 rounded-full bg-black/60 text-white text-[10px] px-1.5 py-0.5"
          title={`Used by ${usageCount} ${usageCount === 1 ? "place" : "places"}`}
        >
          {usageCount}
        </span>
      )}

      {/* Footer */}
      <div className="p-2 bg-surface-subtle">
        <p className="text-xs font-medium text-text-body truncate" title={filename}>
          {filename}
        </p>
        <p className="text-[10px] text-text-muted flex items-center gap-1.5">
          <span>{formatFileSize(sizeBytes)}</span>
          {width && height && (
            <>
              <span className="opacity-40">·</span>
              <span>{width}×{height}</span>
            </>
          )}
        </p>
      </div>
    </div>
  );
}
