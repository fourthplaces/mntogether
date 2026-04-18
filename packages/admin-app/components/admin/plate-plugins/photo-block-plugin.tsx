"use client";

/**
 * PhotoBlockPlugin — inline photo embed within the body.
 *
 * Node type: "photo_block"
 * Void: true (data stored as node attributes, not editable children)
 *
 * Node data: { src, caption, credit, variant: 'c' | 'd', mediaId? }
 * In editor: full-width with variant label. Choose image via the shared
 * <PhotoPickerProvider> picker (see photo-picker-context.tsx).
 * On web-app: PhotoC (float right 58%) or PhotoD (float left 35%).
 */

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { ArrowRightLeft, ImageIcon } from "lucide-react";
import { usePhotoPicker } from "./photo-picker-context";
import type { PickedMedia } from "@/components/admin/MediaPicker";

export const PHOTO_BLOCK_KEY = "photo_block";

interface PhotoBlockData {
  src?: string;
  caption?: string;
  credit?: string;
  variant?: "c" | "d";
  mediaId?: string;
}

export function PhotoBlockElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & PhotoBlockData;
  const variant = data.variant || "c";
  const variantLabel = variant === "c" ? "Medium right" : "Small left";
  const { openPicker } = usePhotoPicker();

  const updateData = useCallback(
    (patch: Partial<TElement & PhotoBlockData>) => {
      const path = editor.api.findPath(element);
      if (path) {
        editor.tf.setNodes(patch as Partial<TElement>, { at: path });
      }
    },
    [editor, element],
  );

  const handlePick = useCallback(
    (picked: PickedMedia) => {
      updateData({ src: picked.url, mediaId: picked.id });
    },
    [updateData],
  );

  const handleOpen = useCallback(() => {
    openPicker({ title: "Choose photo", onSelect: handlePick });
  }, [openPicker, handlePick]);

  const handleRemove = useCallback(() => {
    updateData({ src: "", mediaId: "" });
  }, [updateData]);

  const toggleVariant = useCallback(() => {
    updateData({ variant: variant === "c" ? "d" : "c" });
  }, [updateData, variant]);

  return (
    <PlateElement {...rest} element={element} editor={editor} className="photo-block">
      <div contentEditable={false} onMouseDown={(e) => { if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLButtonElement)) e.preventDefault(); }}>
        <span className="photo-block__variant-label">{variantLabel}</span>

        {data.src ? (
          <div className="relative group">
            {/* eslint-disable-next-line @next/next/no-img-element */}
            <img src={data.src} alt={data.caption || ""} />
            <div className="absolute top-2 right-2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                type="button"
                className="text-xs bg-white/90 hover:bg-white px-2 py-1 rounded border border-border"
                onClick={handleOpen}
              >
                Change
              </button>
              <button
                type="button"
                className="text-xs bg-white/90 hover:bg-white px-2 py-1 rounded border border-border"
                onClick={handleRemove}
              >
                Remove
              </button>
            </div>
          </div>
        ) : (
          <button
            type="button"
            className="photo-block__placeholder w-full flex items-center justify-center gap-2 cursor-pointer hover:bg-muted/40 transition-colors"
            onClick={handleOpen}
          >
            <ImageIcon className="size-4" />
            <span>Choose photo…</span>
          </button>
        )}

        <input
          className="void-input photo-block__caption-text"
          placeholder="Caption"
          value={data.caption || ""}
          onChange={(e) => updateData({ caption: e.target.value })}
        />
        <input
          className="void-input photo-block__credit"
          placeholder="Credit"
          value={data.credit || ""}
          onChange={(e) => updateData({ credit: e.target.value })}
        />

        <div className="block-actions">
          <button type="button" className="block-action-btn" onClick={toggleVariant}>
            <ArrowRightLeft size={12} strokeWidth={2} /> {variant === "c" ? "Small Left" : "Medium Right"}
          </button>
        </div>
      </div>
      {children}
    </PlateElement>
  );
}

export const PhotoBlockPlugin = createPlatePlugin({
  key: PHOTO_BLOCK_KEY,
  node: { isElement: true, isVoid: true, type: PHOTO_BLOCK_KEY },
  render: { node: PhotoBlockElement },
});
