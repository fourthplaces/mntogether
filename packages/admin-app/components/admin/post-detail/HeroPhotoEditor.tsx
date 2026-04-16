"use client";

import * as React from "react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ImageIcon } from "lucide-react";
import { EditorFooter } from "./EditableRow";

type Media = { imageUrl?: string | null; caption?: string | null; credit?: string | null } | null;

export function HeroPhotoEditor({
  media,
  onSave,
}: {
  media: Media;
  onSave: (next: { imageUrl: string | null; caption: string | null; credit: string | null }) => Promise<unknown>;
}) {
  const [open, setOpen] = React.useState(false);
  const [imageUrl, setImageUrl] = React.useState(media?.imageUrl || "");
  const [caption, setCaption] = React.useState(media?.caption || "");
  const [credit, setCredit] = React.useState(media?.credit || "");
  const [saving, setSaving] = React.useState(false);

  React.useEffect(() => {
    if (open) {
      setImageUrl(media?.imageUrl || "");
      setCaption(media?.caption || "");
      setCredit(media?.credit || "");
    }
  }, [open, media]);

  const handleSave = async () => {
    setSaving(true);
    try {
      await onSave({
        imageUrl: imageUrl.trim() || null,
        caption: caption.trim() || null,
        credit: credit.trim() || null,
      });
      setOpen(false);
    } finally {
      setSaving(false);
    }
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
            <label className="block text-xs text-muted-foreground">Image URL</label>
            <Input
              value={imageUrl}
              onChange={(e) => setImageUrl(e.target.value)}
              placeholder="https://…"
              className="text-sm"
            />
            <label className="block text-xs text-muted-foreground">Caption</label>
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
    </div>
  );
}
