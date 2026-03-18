"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const LIST_B_KEY = "list_b";

export function ListBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { items?: string[]; ordered?: boolean };
  const items = data.items || [""];
  const ordered = data.ordered || false;

  const updateData = useCallback(
    (field: string, value: unknown) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  const Tag = ordered ? "ol" : "ul";

  return (
    <PlateElement {...rest} element={element} editor={editor} contentEditable={false}>
      <Tag className="list-b" style={{ listStyle: "none", padding: 0 }}>
        {items.map((item, i) => (
          <li key={i} style={{ display: "flex", gap: "4px", alignItems: "baseline", borderBottom: "1px dotted rgba(0,0,0,0.1)", padding: "8px 0" }}>
            <input className="void-input" value={item} onChange={(e) => { const t = [...items]; t[i] = e.target.value; updateData("items", t); }} placeholder={`Item ${i + 1}`} style={{ flex: 1 }} />
            <button type="button" className="block-action-btn" onClick={() => updateData("items", items.filter((_, j) => j !== i))}>×</button>
          </li>
        ))}
      </Tag>
      <div className="block-actions">
        <button type="button" className="block-action-btn" onClick={() => updateData("items", [...items, ""])}>+ Item</button>
        <button type="button" className="block-action-btn" onClick={() => updateData("ordered", !ordered)}>{ordered ? "→ Unordered" : "→ Ordered"}</button>
      </div>
      {children}
    </PlateElement>
  );
}

export const ListBPlugin = createPlatePlugin({
  key: LIST_B_KEY,
  node: { isElement: true, isVoid: true, type: LIST_B_KEY },
  render: { node: ListBElement },
});
