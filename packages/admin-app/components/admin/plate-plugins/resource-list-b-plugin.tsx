"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Plus, X } from "lucide-react";

export const RESOURCE_LIST_B_KEY = "resource_list_b";

interface ResourceItem { name: string; detail: string; }

export function ResourceListBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { items?: ResourceItem[] };
  const items = data.items || [];

  const updateItems = useCallback(
    (newItems: ResourceItem[]) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ items: newItems } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="resource-list" {...{contentEditable: false} as any}>
      <ul className="list-b" style={{ listStyle: "none", padding: 0 }}>
        {items.map((item, i) => (
          <li key={i} style={{ display: "flex", gap: "8px", alignItems: "baseline", borderBottom: "1px dotted rgba(0,0,0,0.1)", padding: "8px 0" }}>
            <input className="void-input" value={item.name} onChange={(e) => { const t = [...items]; t[i] = { ...t[i], name: e.target.value }; updateItems(t); }} placeholder="Name" style={{ flex: "0 0 30%", fontWeight: 700 }} />
            <span style={{ color: "var(--pebble)" }}>·</span>
            <input className="void-input" value={item.detail} onChange={(e) => { const t = [...items]; t[i] = { ...t[i], detail: e.target.value }; updateItems(t); }} placeholder="Detail" style={{ flex: 1 }} />
            <button type="button" className="block-action-btn" onClick={() => updateItems(items.filter((_, j) => j !== i))}><X size={12} strokeWidth={2} /></button>
          </li>
        ))}
      </ul>
      <div className="block-actions">
        <button type="button" className="block-action-btn" onClick={() => updateItems([...items, { name: "", detail: "" }])}><Plus size={12} strokeWidth={2} /> Item</button>
      </div>
      {children}
    </PlateElement>
  );
}

export const ResourceListBPlugin = createPlatePlugin({
  key: RESOURCE_LIST_B_KEY,
  node: { isElement: true, isVoid: true, type: RESOURCE_LIST_B_KEY },
  render: { node: ResourceListBElement },
});
