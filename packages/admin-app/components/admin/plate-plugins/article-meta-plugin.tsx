"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Minus, Plus } from "lucide-react";

export const ARTICLE_META_KEY = "article_meta";

export function ArticleMetaElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { parts?: string[] };
  const parts = data.parts || [""];

  const updateParts = useCallback(
    (newParts: string[]) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ parts: newParts } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="article-meta" {...{contentEditable: false} as any}>
      {parts.map((part, i) => (
        <span key={i}>
          {i > 0 && <span className="sep" style={{ margin: "0 0.6em" }}>·</span>}
          <input className="void-input" value={part} onChange={(e) => { const p = [...parts]; p[i] = e.target.value; updateParts(p); }} placeholder={i === 0 ? "Byline" : i === 1 ? "Date" : "Location"} style={{ display: "inline", width: "auto", minWidth: "80px" }} />
        </span>
      ))}
      <div className="block-actions">
        <button type="button" className="block-action-btn" onClick={() => updateParts([...parts, ""])}><Plus size={12} strokeWidth={2} /> Part</button>
        {parts.length > 1 && <button type="button" className="block-action-btn" onClick={() => updateParts(parts.slice(0, -1))}><Minus size={12} strokeWidth={2} /> Part</button>}
      </div>
      {children}
    </PlateElement>
  );
}

export const ArticleMetaPlugin = createPlatePlugin({
  key: ARTICLE_META_KEY,
  node: { isElement: true, isVoid: true, type: ARTICLE_META_KEY },
  render: { node: ArticleMetaElement },
});
