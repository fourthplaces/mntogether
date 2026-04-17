"use client";

/**
 * MediaPicker — the one modal every image field in the admin opens.
 *
 *   <MediaPicker
 *     open={open}
 *     onOpenChange={setOpen}
 *     onSelect={(media) => { ...write media_id + url somewhere }}
 *     contentTypeFilter="image/*"
 *     usageContext="post_hero"
 *   />
 *
 * What it offers, in one place:
 *   - Upload zone (drag-drop or click-to-browse). Newly uploaded items are
 *     auto-selected if `autoSelectOnUpload` is set.
 *   - Search (filename + alt text, debounced).
 *   - Grid of existing media. Click a card to select + insert.
 *   - Footer with Cancel.
 *
 * Library browsing and uploading live in the same surface so editors don't
 * have to leave the post/widget they're editing. The `useMediaUpload` hook
 * and `MediaUploadZone` component are reused as-is.
 */

import * as React from "react";
import { useQuery } from "urql";
import Image from "next/image";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Search, FileIcon, Check } from "lucide-react";
import { cn } from "@/lib/utils";
import { MediaUploadZone } from "./MediaUploadZone";
import { useMediaUpload } from "@/lib/hooks/useMediaUpload";
import { MediaLibraryQuery } from "@/lib/graphql/media";

export type PickedMedia = {
  id: string;
  filename: string;
  contentType: string;
  url: string;
  altText?: string | null;
  width?: number | null;
  height?: number | null;
};

export type MediaPickerProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (media: PickedMedia) => void;
  /** e.g. "image/*" — filters the library view + constrains upload input. */
  contentTypeFilter?: string;
  /** Human-readable context label (e.g. "Post hero photo") for the dialog title. */
  title?: string;
};

const PAGE_SIZE = 24;

export function MediaPicker({
  open,
  onOpenChange,
  onSelect,
  contentTypeFilter = "image/*",
  title = "Choose media",
}: MediaPickerProps) {
  const [search, setSearch] = React.useState("");
  const debouncedSearch = useDebounced(search, 200);

  // Reset search when the picker closes so reopening doesn't remember state.
  React.useEffect(() => {
    if (!open) setSearch("");
  }, [open]);

  const [{ data, fetching }, refetch] = useQuery({
    query: MediaLibraryQuery,
    variables: {
      limit: PAGE_SIZE,
      offset: 0,
      contentType: contentTypeFilter ? imageContentTypePrefix(contentTypeFilter) : undefined,
      search: debouncedSearch || undefined,
      unusedOnly: false,
    },
    pause: !open,
    requestPolicy: "cache-and-network",
  });

  const items = data?.mediaLibrary?.media ?? [];

  const { uploadFiles } = useMediaUpload();

  const handleFiles = async (files: File[]) => {
    const uploaded = await uploadFiles(files);
    // Auto-select the first successful upload — editors who upload generally
    // want the thing they just uploaded inserted immediately.
    const first = uploaded.find((m): m is NonNullable<typeof m> => m != null);
    if (first) {
      onSelect({
        id: first.id,
        filename: first.filename,
        contentType: first.contentType,
        url: first.url,
        altText: first.altText,
        width: first.width,
        height: first.height,
      });
      onOpenChange(false);
    } else {
      // Either no file was usable, or all uploads errored. Refresh the grid
      // so any partial successes appear.
      refetch({ requestPolicy: "network-only" });
    }
  };

  const handlePick = (item: (typeof items)[number]) => {
    onSelect({
      id: item.id,
      filename: item.filename,
      contentType: item.contentType,
      url: item.url,
      altText: item.altText,
      width: item.width,
      height: item.height,
    });
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[960px] max-h-[90vh] flex flex-col gap-3 p-5">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-base font-semibold">{title}</h2>
        </div>

        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search filename or alt text…"
              className="pl-8 text-sm"
            />
          </div>
        </div>

        <MediaUploadZone
          onFiles={handleFiles}
          accept={contentTypeFilter === "image/*" ? undefined : contentTypeFilter}
          className="py-4"
        />

        <div className="flex-1 overflow-y-auto min-h-[240px]">
          {fetching && items.length === 0 ? (
            <div className="text-sm text-muted-foreground italic py-8 text-center">
              Loading…
            </div>
          ) : items.length === 0 ? (
            <div className="text-sm text-muted-foreground italic py-8 text-center">
              {debouncedSearch
                ? `No media matches "${debouncedSearch}".`
                : "No media in the library yet. Upload some above."}
            </div>
          ) : (
            <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 gap-3">
              {items.map((item) => (
                <MediaTile key={item.id} item={item} onClick={() => handlePick(item)} />
              ))}
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-border pt-3">
          <Button variant="outline" size="sm" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function MediaTile({
  item,
  onClick,
}: {
  item: {
    id: string;
    url: string;
    filename: string;
    contentType: string;
    width?: number | null;
    height?: number | null;
    usageCount?: number | null;
  };
  onClick: () => void;
}) {
  const isImage = item.contentType.startsWith("image/");
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        // ring-inset draws the hover outline INSIDE the tile so the top/left
        // edges aren't clipped by the grid's scroll container.
        "group relative aspect-square rounded-md overflow-hidden border border-border bg-muted/20 hover:ring-2 hover:ring-inset hover:ring-primary/60 transition-all",
      )}
      title={item.filename}
    >
      {isImage ? (
        <Image
          src={item.url}
          alt={item.filename}
          fill
          sizes="200px"
          className="object-cover"
          unoptimized
        />
      ) : (
        <div className="flex items-center justify-center h-full text-muted-foreground">
          <FileIcon className="size-8" />
        </div>
      )}

      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/70 to-transparent p-1.5 text-[10px] text-white opacity-0 group-hover:opacity-100 transition-opacity">
        <div className="truncate">{item.filename}</div>
        {item.width && item.height && (
          <div className="text-white/70">{item.width}×{item.height}</div>
        )}
      </div>

      {item.usageCount != null && item.usageCount > 0 && (
        <span
          className="absolute top-1 right-1 rounded-full bg-black/60 text-white text-[10px] px-1.5 py-0.5"
          title={`Used by ${item.usageCount} place${item.usageCount === 1 ? "" : "s"}`}
        >
          <Check className="inline size-2.5 mr-0.5" />
          {item.usageCount}
        </span>
      )}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * The server's mediaLibrary filter matches content_type as a LIKE prefix, so
 * we need to send the concrete prefix (e.g. "image/") rather than the
 * shorthand glob ("image/*").
 */
function imageContentTypePrefix(filter: string): string | undefined {
  if (!filter) return undefined;
  if (filter.endsWith("/*")) return filter.slice(0, filter.length - 1); // "image/*" → "image/"
  return filter;
}

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = React.useState(value);
  React.useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}
