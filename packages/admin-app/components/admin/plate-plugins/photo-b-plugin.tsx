"use client";

/**
 * PhotoBPlugin — full-bleed photo block.
 *
 * Node type: "photo_b"
 * Void: true
 *
 * Node data: { src, caption, credit, mediaId? }
 *
 * Image selection routes through the shared picker from
 * `<PhotoPickerProvider>`. See photo-a-plugin.tsx for the shared pattern
 * and photo-picker-context.tsx for the rationale.
 */

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { ImageIcon } from "lucide-react";
import { usePhotoPicker } from "./photo-picker-context";
import type { PickedMedia } from "@/components/admin/MediaPicker";

export const PHOTO_B_KEY = "photo_b";

type PhotoBData = TElement & {
  src?: string;
  caption?: string;
  credit?: string;
  mediaId?: string;
};

export function PhotoBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as PhotoBData;
  const { openPicker } = usePhotoPicker();

  const updateData = useCallback(
    (patch: Partial<PhotoBData>) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes(patch as Partial<TElement>, { at: path });
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
    openPicker({ title: "Choose full-bleed photo", onSelect: handlePick });
  }, [openPicker, handlePick]);

  const handleRemove = useCallback(() => {
    updateData({ src: "", mediaId: "" });
  }, [updateData]);

  return (
    <PlateElement
      {...rest}
      element={element}
      editor={editor}
      className="photo-b"
    >
      <div
        contentEditable={false}
        onMouseDown={(e) => {
          if (
            !(e.target instanceof HTMLInputElement || e.target instanceof HTMLButtonElement)
          )
            e.preventDefault();
        }}
      >
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
            <span>Choose full-bleed photo…</span>
          </button>
        )}
        <div style={{ padding: "8px 0" }}>
          <input
            className="void-input"
            placeholder="Caption"
            value={data.caption || ""}
            onChange={(e) => updateData({ caption: e.target.value })}
            style={{ fontStyle: "italic", fontSize: "0.85rem", color: "var(--slate)" }}
          />
          <input
            className="void-input"
            placeholder="Credit"
            value={data.credit || ""}
            onChange={(e) => updateData({ credit: e.target.value })}
            style={{ fontSize: "0.72rem", color: "var(--pebble)" }}
          />
        </div>
      </div>
      {children}
    </PlateElement>
  );
}

export const PhotoBPlugin = createPlatePlugin({
  key: PHOTO_B_KEY,
  node: { isElement: true, isVoid: true, type: PHOTO_B_KEY },
  render: { node: PhotoBElement },
});
