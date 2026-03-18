"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const PHOTO_B_KEY = "photo_b";

export function PhotoBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { src?: string; caption?: string; credit?: string };

  const updateData = useCallback(
    (field: string, value: string) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="photo-b" {...{contentEditable: false} as any}>
      {data.src ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img src={data.src} alt={data.caption || ""} />
      ) : (
        <div className="photo-block__placeholder">Full-bleed photo</div>
      )}
      <div style={{ padding: "8px 0" }}>
        <input className="void-input" placeholder="Caption" value={data.caption || ""} onChange={(e) => updateData("caption", e.target.value)} style={{ fontStyle: "italic", fontSize: "0.85rem", color: "var(--slate)" }} />
        <input className="void-input" placeholder="Credit" value={data.credit || ""} onChange={(e) => updateData("credit", e.target.value)} style={{ fontSize: "0.72rem", color: "var(--pebble)" }} />
      </div>
      <input className="void-input" placeholder="Image URL" value={data.src || ""} onChange={(e) => updateData("src", e.target.value)} style={{ fontFamily: "var(--font-mono)", fontSize: "0.72rem", color: "var(--pebble)" }} />
      {children}
    </PlateElement>
  );
}

export const PhotoBPlugin = createPlatePlugin({
  key: PHOTO_B_KEY,
  node: { isElement: true, isVoid: true, type: PHOTO_B_KEY },
  render: { node: PhotoBElement },
});
