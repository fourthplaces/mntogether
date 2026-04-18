"use client";

/**
 * PhotoAPlugin — full-width photo block.
 *
 * Node type: "photo_a"
 * Void: true (image data lives as node attributes, no text children)
 *
 * Node data: { src, caption, credit, mediaId? }
 *
 * Image source always flows through the Media Library: the "Choose photo"
 * button opens the shared picker hosted by `<PhotoPickerProvider>` at the
 * editor level. Raw URL paste is not supported — every body image needs to
 * be tracked for usage counts + consistent storage, which means going
 * through the library.
 */

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { ImageIcon } from "lucide-react";
import { usePhotoPicker } from "./photo-picker-plugin";
import type { PickedMedia } from "@/components/admin/MediaPicker";

export const PHOTO_A_KEY = "photo_a";

type PhotoAData = TElement & {
  src?: string;
  caption?: string;
  credit?: string;
  mediaId?: string;
};

export function PhotoAElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as PhotoAData;
  const { openPicker } = usePhotoPicker();

  const updateData = useCallback(
    (patch: Partial<PhotoAData>) => {
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
    openPicker({ title: "Choose full-width photo", onSelect: handlePick });
  }, [openPicker, handlePick]);

  const handleRemove = useCallback(() => {
    updateData({ src: "", mediaId: "" });
  }, [updateData]);

  return (
    <PlateElement
      {...rest}
      element={element}
      editor={editor}
      className="photo-a"
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
            <span>Choose full-width photo…</span>
          </button>
        )}
        <div className="photo-a__caption">
          <input
            className="void-input photo-a__caption-text"
            placeholder="Caption"
            value={data.caption || ""}
            onChange={(e) => updateData({ caption: e.target.value })}
          />
          <input
            className="void-input photo-a__credit"
            placeholder="Credit"
            value={data.credit || ""}
            onChange={(e) => updateData({ credit: e.target.value })}
            style={{ textAlign: "right", maxWidth: "200px" }}
          />
        </div>
      </div>
      {children}
    </PlateElement>
  );
}

export const PhotoAPlugin = createPlatePlugin({
  key: PHOTO_A_KEY,
  node: { isElement: true, isVoid: true, type: PHOTO_A_KEY },
  render: { node: PhotoAElement },
});
