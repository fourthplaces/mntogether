"use client";

/**
 * MediaDetailPanel — slide-in side panel showing everything about a
 * single media item:
 *
 *   - Large preview
 *   - Editable alt text + filename (debounced save)
 *   - Read-only metadata (type, size, dimensions, storage key, uploader)
 *   - Usage list: "used by Mountain Lake Food Shelf (post)" etc.,
 *     each linking back to the entity in the admin
 *   - Actions: Copy URL, Download, Delete (confirmed, warns if in use)
 *
 * Opens via shadcn Sheet. Caller controls open/close + selected media.
 */

import * as React from "react";
import Link from "next/link";
import Image from "next/image";
import { useQuery, useMutation } from "urql";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Copy, Download, Trash2, ExternalLink, FileIcon, AlertTriangle } from "lucide-react";
import {
  MediaUsageQuery,
  UpdateMediaMetadataMutation,
  DeleteMediaMutation,
} from "@/lib/graphql/media";

type DetailMedia = {
  id: string;
  filename: string;
  contentType: string;
  sizeBytes: number;
  url: string;
  storageKey: string;
  altText?: string | null;
  width?: number | null;
  height?: number | null;
  createdAt: string;
  updatedAt?: string;
  usageCount?: number | null;
};

export function MediaDetailPanel({
  media,
  open,
  onOpenChange,
  onDeleted,
}: {
  media: DetailMedia | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDeleted?: (id: string) => void;
}) {
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="sm:max-w-[480px] flex flex-col p-0 gap-0">
        {media ? (
          <DetailBody media={media} onDeleted={onDeleted} onClose={() => onOpenChange(false)} />
        ) : (
          <div className="p-6 text-sm text-muted-foreground italic">No media selected.</div>
        )}
      </SheetContent>
    </Sheet>
  );
}

function DetailBody({
  media,
  onDeleted,
  onClose,
}: {
  media: DetailMedia;
  onDeleted?: (id: string) => void;
  onClose: () => void;
}) {
  const [altText, setAltText] = React.useState(media.altText || "");
  const [filename, setFilename] = React.useState(media.filename);
  const [saveStatus, setSaveStatus] = React.useState<"idle" | "saving" | "saved">("idle");
  const [copied, setCopied] = React.useState(false);
  const [confirmDelete, setConfirmDelete] = React.useState(false);

  // Re-seed local state when the selected media changes (different id).
  React.useEffect(() => {
    setAltText(media.altText || "");
    setFilename(media.filename);
    setSaveStatus("idle");
    setConfirmDelete(false);
  }, [media.id, media.altText, media.filename]);

  const [{ data: usageData }] = useQuery({
    query: MediaUsageQuery,
    variables: { mediaId: media.id },
    requestPolicy: "cache-and-network",
  });
  const usage = usageData?.mediaUsage ?? [];

  const [, updateMetadata] = useMutation(UpdateMediaMetadataMutation);
  const [, deleteMedia] = useMutation(DeleteMediaMutation);

  // Debounced save of alt text + filename.
  const saveTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  React.useEffect(() => {
    if (altText === (media.altText || "") && filename === media.filename) {
      setSaveStatus("idle");
      return;
    }
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    setSaveStatus("saving");
    saveTimerRef.current = setTimeout(async () => {
      await updateMetadata(
        {
          id: media.id,
          altText: altText || null,
          filename: filename || null,
        },
        { additionalTypenames: ["Media", "MediaConnection"] },
      );
      setSaveStatus("saved");
      setTimeout(() => setSaveStatus("idle"), 1500);
    }, 600);
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
  }, [altText, filename, media.id, media.altText, media.filename, updateMetadata]);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(media.url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const handleDelete = async () => {
    await deleteMedia({ id: media.id }, { additionalTypenames: ["Media", "MediaConnection"] });
    onDeleted?.(media.id);
    onClose();
  };

  const isImage = media.contentType.startsWith("image/");
  const usageCount = usage.length;

  return (
    <>
      <SheetHeader className="px-5 pt-5 pb-3 border-b border-border">
        <SheetTitle className="text-base">Media details</SheetTitle>
      </SheetHeader>

      <div className="flex-1 overflow-y-auto">
        {/* Preview */}
        <div className="bg-muted/30 p-5 border-b border-border">
          <div className="relative aspect-video bg-background rounded-md overflow-hidden border border-border">
            {isImage ? (
              <Image
                src={media.url}
                alt={media.altText || media.filename}
                fill
                sizes="480px"
                className="object-contain"
                unoptimized
              />
            ) : (
              <div className="flex items-center justify-center h-full text-muted-foreground">
                <FileIcon className="size-12" />
              </div>
            )}
          </div>
        </div>

        <div className="px-5 py-4 space-y-4">
          {/* Editable fields */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="block text-xs uppercase tracking-wide text-muted-foreground">
                Filename
              </label>
              {saveStatus !== "idle" && (
                <span className="text-[10px] text-muted-foreground">
                  {saveStatus === "saving" ? "Saving…" : "Saved"}
                </span>
              )}
            </div>
            <Input
              value={filename}
              onChange={(e) => setFilename(e.target.value)}
              className="text-sm"
            />

            <label className="block text-xs uppercase tracking-wide text-muted-foreground pt-1">
              Alt text
              <span className="text-muted-foreground/60 normal-case ml-1">(for accessibility)</span>
            </label>
            <textarea
              value={altText}
              onChange={(e) => setAltText(e.target.value)}
              placeholder="Describe the image for screen readers…"
              rows={3}
              className="w-full rounded border border-border bg-card px-2 py-1.5 text-sm"
            />
          </div>

          {/* Read-only metadata */}
          <dl className="text-xs space-y-1.5 border-t border-border pt-3">
            <Row label="Type">{media.contentType}</Row>
            <Row label="Size">{formatFileSize(media.sizeBytes)}</Row>
            {media.width != null && media.height != null && (
              <Row label="Dimensions">{media.width}×{media.height}</Row>
            )}
            <Row label="Uploaded">{new Date(media.createdAt).toLocaleString()}</Row>
            {media.updatedAt && media.updatedAt !== media.createdAt && (
              <Row label="Modified">{new Date(media.updatedAt).toLocaleString()}</Row>
            )}
            <Row label="Storage key">
              <span className="font-mono text-[10px]">{media.storageKey}</span>
            </Row>
            <Row label="URL">
              <a
                href={media.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-link hover:text-link-hover font-mono text-[10px] truncate max-w-[280px] inline-block align-bottom"
              >
                {media.url}
              </a>
            </Row>
          </dl>

          {/* Usage */}
          <div className="border-t border-border pt-3">
            <div className="flex items-center justify-between mb-2">
              <h3 className="text-xs uppercase tracking-wide text-muted-foreground font-semibold">
                Used by
              </h3>
              <Badge variant="secondary" className="text-[10px]">
                {usageCount} {usageCount === 1 ? "place" : "places"}
              </Badge>
            </div>
            {usageCount === 0 ? (
              <p className="text-xs text-muted-foreground italic">
                Not currently used anywhere in the admin.
              </p>
            ) : (
              <ul className="space-y-1.5">
                {usage.map((u, i) => (
                  <UsageItem key={i} usage={u} />
                ))}
              </ul>
            )}
          </div>
        </div>
      </div>

      {/* Footer actions */}
      <div className="border-t border-border px-5 py-3 flex items-center justify-between bg-muted/10">
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={handleCopy}>
            <Copy className="size-3.5 mr-1.5" />
            {copied ? "Copied!" : "Copy URL"}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            render={<a href={media.url} download={media.filename} target="_blank" rel="noopener noreferrer" />}
          >
            <Download className="size-3.5 mr-1.5" />
            Download
          </Button>
        </div>

        {confirmDelete ? (
          <div className="flex items-center gap-1">
            {usageCount > 0 && (
              <span className="text-[10px] text-amber-700 flex items-center gap-1 mr-1">
                <AlertTriangle className="size-3" />
                In use!
              </span>
            )}
            <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(false)}>
              Cancel
            </Button>
            <Button variant="destructive" size="sm" onClick={handleDelete}>
              Delete
            </Button>
          </div>
        ) : (
          <Button
            variant="ghost"
            size="sm"
            className="text-destructive hover:text-destructive"
            onClick={() => setConfirmDelete(true)}
          >
            <Trash2 className="size-3.5 mr-1.5" />
            Delete
          </Button>
        )}
      </div>
    </>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <dt className="text-muted-foreground flex-shrink-0">{label}</dt>
      <dd className="text-foreground text-right min-w-0">{children}</dd>
    </div>
  );
}

function UsageItem({
  usage,
}: {
  usage: { referenceableType: string; referenceableId: string; title: string; fieldKey?: string | null };
}) {
  const { label, href, icon } = resolveUsageLink(usage);
  const content = (
    <div className="flex items-center gap-2 py-1.5 px-2 rounded hover:bg-muted/40 transition-colors">
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground w-16 flex-shrink-0">
        {label}
      </span>
      <span className="text-sm text-foreground flex-1 truncate">{usage.title}</span>
      {icon}
    </div>
  );
  return (
    <li>
      {href ? (
        <Link href={href}>{content}</Link>
      ) : (
        content
      )}
    </li>
  );
}

function resolveUsageLink(u: { referenceableType: string; referenceableId: string }) {
  switch (u.referenceableType) {
    case "post_hero":
    case "post_person":
    case "post_body":
      return {
        label: u.referenceableType.replace("post_", ""),
        href: `/admin/posts/${u.referenceableId}`,
        icon: <ExternalLink className="size-3 text-muted-foreground opacity-0 group-hover:opacity-100" />,
      };
    case "widget":
      return {
        label: "widget",
        href: `/admin/widgets/${u.referenceableId}`,
        icon: <ExternalLink className="size-3 text-muted-foreground" />,
      };
    case "organization_logo":
      return {
        label: "org logo",
        href: `/admin/organizations/${u.referenceableId}`,
        icon: <ExternalLink className="size-3 text-muted-foreground" />,
      };
    default:
      return { label: u.referenceableType, href: null, icon: null };
  }
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
