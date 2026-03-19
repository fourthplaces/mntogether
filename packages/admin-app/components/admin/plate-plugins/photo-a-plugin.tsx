"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const PHOTO_A_KEY = "photo_a";

export function PhotoAElement(props: PlateElementProps) {
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
    <PlateElement {...rest} element={element} editor={editor} className="photo-a">
      <div contentEditable={false} onMouseDown={(e) => { if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLButtonElement)) e.preventDefault(); }}>
        {data.src ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={data.src} alt={data.caption || ""} />
        ) : (
          <div className="photo-block__placeholder">Full-width photo</div>
        )}
        <div className="photo-a__caption">
          <input className="void-input photo-a__caption-text" placeholder="Caption" value={data.caption || ""} onChange={(e) => updateData("caption", e.target.value)} />
          <input className="void-input photo-a__credit" placeholder="Credit" value={data.credit || ""} onChange={(e) => updateData("credit", e.target.value)} style={{ textAlign: "right", maxWidth: "200px" }} />
        </div>
        <input className="void-input" placeholder="Image URL" value={data.src || ""} onChange={(e) => updateData("src", e.target.value)} style={{ fontFamily: "var(--font-mono)", fontSize: "0.72rem", color: "var(--pebble)" }} />
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
