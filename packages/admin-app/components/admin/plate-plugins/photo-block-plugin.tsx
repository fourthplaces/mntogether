"use client";

/**
 * PhotoBlockPlugin — inline photo embed within the body.
 *
 * Node type: "photo_block"
 * Void: true (data stored as node attributes, not editable children)
 *
 * Node data: { src, caption, credit, variant: 'c' | 'd' }
 * In editor: full-width with variant label. Editable inputs for URL/caption/credit.
 * On web-app: PhotoC (float right 58%) or PhotoD (float left 35%).
 */

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const PHOTO_BLOCK_KEY = "photo_block";

interface PhotoBlockData {
  src?: string;
  caption?: string;
  credit?: string;
  variant?: "c" | "d";
}

export function PhotoBlockElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & PhotoBlockData;
  const variant = data.variant || "c";
  const variantLabel = variant === "c" ? "Medium right" : "Small left";

  const updateData = useCallback(
    (field: string, value: string) => {
      const path = editor.api.findPath(element);
      if (path) {
        editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
      }
    },
    [editor, element]
  );

  const toggleVariant = useCallback(() => {
    const path = editor.api.findPath(element);
    if (path) {
      editor.tf.setNodes(
        { variant: variant === "c" ? "d" : "c" } as Partial<TElement>,
        { at: path }
      );
    }
  }, [editor, element, variant]);

  return (
    <PlateElement {...rest} element={element} editor={editor} className="photo-block" contentEditable={false}>
      <span className="photo-block__variant-label">{variantLabel}</span>

      {data.src ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={data.src} alt={data.caption || ""} />
      ) : (
        <div className="photo-block__placeholder">Click to add image URL</div>
      )}

      <input
        className="void-input photo-block__caption-text"
        placeholder="Caption"
        value={data.caption || ""}
        onChange={(e) => updateData("caption", e.target.value)}
      />
      <input
        className="void-input photo-block__credit"
        placeholder="Credit"
        value={data.credit || ""}
        onChange={(e) => updateData("credit", e.target.value)}
      />
      <input
        className="void-input"
        placeholder="Image URL"
        value={data.src || ""}
        onChange={(e) => updateData("src", e.target.value)}
        style={{ fontFamily: "var(--font-mono)", fontSize: "0.72rem" }}
      />

      <div className="block-actions">
        <button type="button" className="block-action-btn" onClick={toggleVariant}>
          {variant === "c" ? "→ Small Left" : "→ Medium Right"}
        </button>
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
