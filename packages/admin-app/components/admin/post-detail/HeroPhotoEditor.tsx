"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ImageIcon } from "lucide-react";
import { EditorFooter } from "./EditableRow";
import { MediaPicker, type PickedMedia } from "@/components/admin/MediaPicker";

type Media = {
  imageUrl?: string | null;
  caption?: string | null;
  credit?: string | null;
  mediaId?: string | null;
} | null;

export function HeroPhotoEditor({
  media,
  onSave,
}: {
  media: Media;
  onSave: (next: {
    imageUrl: string | null;
    caption: string | null;
    credit: string | null;
    mediaId: string | null;
  }) => Promise<unknown>;
}) {
  const [open, setOpen] = React.useState(false);
  const [pickerOpen, setPickerOpen] = React.useState(false);
  const [imageUrl, setImageUrl] = React.useState(media?.imageUrl || "");
  const [caption, setCaption] = React.useState(media?.caption || "");
  const [credit, setCredit] = React.useState(media?.credit || "");
  const [mediaId, setMediaId] = React.useState<string | null>(media?.mediaId ?? null);
  const [saving, setSaving] = React.useState(false);

  // Re-seed draft state from the server's `media` prop whenever it changes
  // (e.g. after a successful save triggers a refetch). We deliberately do
  // NOT key this on `open` — the parent Popover occasionally closes
  // unexpectedly (e.g. when a nested Dialog like MediaPicker grabs focus),
  // and re-seeding on every re-open would erase an in-flight pick.
  const mediaKey = `${media?.mediaId ?? ""}|${media?.imageUrl ?? ""}|${media?.caption ?? ""}|${media?.credit ?? ""}`;
  React.useEffect(() => {
    setImageUrl(media?.imageUrl || "");
    setCaption(media?.caption || "");
    setCredit(media?.credit || "");
    setMediaId(media?.mediaId ?? null);
    // Intentionally depend on the serialized media values so the effect
    // re-runs only on real server changes, not on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mediaKey]);

  const handlePick = (picked: PickedMedia) => {
    setMediaId(picked.id);
    setImageUrl(picked.url);
    // If the parent popover closed when the picker grabbed focus, re-open
    // it now so the user can see the preview + caption/credit fields and
    // click Save. Scheduled as a microtask so the picker's own close
    // animation finishes first.
    setTimeout(() => setOpen(true), 0);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({
        imageUrl: imageUrl.trim() || null,
        caption: caption.trim() || null,
        credit: credit.trim() || null,
        mediaId: mediaId,
      });
      setOpen(false);
    } finally {
      setSaving(false);
    }
  };

  const clearPhoto = () => {
    setImageUrl("");
    setMediaId(null);
  };

  return (
    <div className="border border-border rounded-lg overflow-hidden">
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger
          render={
            <button
              type="button"
              className="block w-full text-left hover:bg-muted/20 transition-colors"
            />
          }
        >
          {media?.imageUrl ? (
            <div>
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={media.imageUrl}
                alt={media.caption || "Hero photo"}
                className="w-full object-cover"
                style={{ maxHeight: "20rem" }}
              />
              <div className="px-3 py-2 text-xs text-muted-foreground">
                {media.caption || <span className="italic">No caption</span>}
                {media.credit && <span className="text-muted-foreground/70 ml-1">— {media.credit}</span>}
              </div>
            </div>
          ) : (
            <div className="flex items-center gap-3 p-6 text-muted-foreground">
              <ImageIcon className="w-5 h-5" />
              <span className="italic text-sm">No hero photo — click to add</span>
            </div>
          )}
        </PopoverTrigger>
        <PopoverContent className="w-96" align="start">
          <div className="space-y-2">
            <div className="text-xs uppercase tracking-wide text-muted-foreground mb-1">Hero Photo</div>

            {imageUrl ? (
              <div className="relative rounded-md overflow-hidden border border-border">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img src={imageUrl} alt="" className="w-full object-cover" style={{ maxHeight: "10rem" }} />
                <Button
                  variant="ghost"
                  size="xs"
                  className="absolute top-1 right-1 bg-background/80 hover:bg-background"
                  onClick={clearPhoto}
                >
                  Remove
                </Button>
              </div>
            ) : (
              <div className="rounded-md border border-dashed border-border p-4 flex flex-col items-center justify-center gap-1 text-muted-foreground">
                <ImageIcon className="size-5" />
                <span className="text-xs italic">No image selected</span>
              </div>
            )}

            <Button
              variant="outline"
              size="sm"
              className="w-full"
              onClick={() => setPickerOpen(true)}
            >
              <ImageIcon className="size-3.5 mr-1.5" />
              {imageUrl ? "Change image" : "Choose image"}
            </Button>

            <label className="block text-xs text-muted-foreground mt-2">Caption</label>
            <Input
              value={caption}
              onChange={(e) => setCaption(e.target.value)}
              placeholder="Short descriptive caption"
              className="text-sm"
            />
            <label className="block text-xs text-muted-foreground">Credit</label>
            <Input
              value={credit}
              onChange={(e) => setCredit(e.target.value)}
              placeholder="Photographer or source"
              className="text-sm"
            />

            <EditorFooter
              onSave={handleSave}
              onCancel={() => setOpen(false)}
              saving={saving}
            />
          </div>
        </PopoverContent>
      </Popover>

      <MediaPicker
        open={pickerOpen}
        onOpenChange={setPickerOpen}
        onSelect={handlePick}
        title="Choose hero photo"
      />
    </div>
  );
}
