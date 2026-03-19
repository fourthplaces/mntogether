"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Minus, Plus } from "lucide-react";

export const KICKER_A_KEY = "kicker_a";

export function KickerAElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { tags?: string[] };
  const tags = data.tags || [""];

  const updateTags = useCallback(
    (newTags: string[]) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ tags: newTags } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="kicker-a">
      <div contentEditable={false} onMouseDown={(e) => { if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLButtonElement)) e.preventDefault(); }}>
        {tags.map((tag, i) => (
          <span key={i}>
            {i > 0 && <span className="sep" style={{ color: "var(--pebble)", margin: "0 0.5em" }}>·</span>}
            <input className="void-input" value={tag} onChange={(e) => { const t = [...tags]; t[i] = e.target.value; updateTags(t); }} placeholder="Tag" style={{ display: "inline", width: "auto", minWidth: "60px", fontSize: "0.82rem" }} />
          </span>
        ))}
        <div className="block-actions">
          <button type="button" className="block-action-btn" onClick={() => updateTags([...tags, ""])}><Plus size={12} strokeWidth={2} /> Tag</button>
          {tags.length > 1 && <button type="button" className="block-action-btn" onClick={() => updateTags(tags.slice(0, -1))}><Minus size={12} strokeWidth={2} /> Tag</button>}
        </div>
      </div>
      {children}
    </PlateElement>
  );
}

export const KickerAPlugin = createPlatePlugin({
  key: KICKER_A_KEY,
  node: { isElement: true, isVoid: true, type: KICKER_A_KEY },
  render: { node: KickerAElement },
});
