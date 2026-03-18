"use client";

/**
 * ResourceListPlugin — name · detail pair list.
 *
 * Node type: "resource_list"
 * Void: true (data stored as node attributes)
 *
 * Node data: { items: Array<{ name, detail }> }
 */

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const RESOURCE_LIST_KEY = "resource_list";

interface ResourceItem {
  name: string;
  detail: string;
}

interface ResourceListData {
  items?: ResourceItem[];
}

export function ResourceListElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & ResourceListData;
  const items = data.items || [];

  const updateItems = useCallback(
    (newItems: ResourceItem[]) => {
      const path = editor.api.findPath(element);
      if (path) {
        editor.tf.setNodes({ items: newItems } as Partial<TElement>, { at: path });
      }
    },
    [editor, element]
  );

  const addItem = () => updateItems([...items, { name: "", detail: "" }]);

  const removeItem = (index: number) => {
    updateItems(items.filter((_, i) => i !== index));
  };

  const updateItem = (index: number, field: keyof ResourceItem, value: string) => {
    const newItems = items.map((item, i) =>
      i === index ? { ...item, [field]: value } : item
    );
    updateItems(newItems);
  };

  return (
    <PlateElement {...rest} element={element} editor={editor} className="resource-list" contentEditable={false}>
      {items.map((item, i) => (
        <div key={i} className="resource-list__item" style={{ display: "flex", gap: "8px", alignItems: "baseline" }}>
          <input
            className="void-input resource-list__name"
            value={item.name}
            onChange={(e) => updateItem(i, "name", e.target.value)}
            placeholder="Name"
            style={{ flex: "0 0 30%", fontWeight: 700 }}
          />
          <span className="resource-list__sep">·</span>
          <input
            className="void-input resource-list__detail"
            value={item.detail}
            onChange={(e) => updateItem(i, "detail", e.target.value)}
            placeholder="Detail"
            style={{ flex: 1 }}
          />
          <button
            type="button"
            className="block-action-btn"
            onClick={() => removeItem(i)}
            title="Remove item"
          >
            ×
          </button>
        </div>
      ))}

      <div className="block-actions">
        <button type="button" className="block-action-btn" onClick={addItem}>
          + Add Item
        </button>
      </div>

      {children}
    </PlateElement>
  );
}

export const ResourceListPlugin = createPlatePlugin({
  key: RESOURCE_LIST_KEY,
  node: { isElement: true, isVoid: true, type: RESOURCE_LIST_KEY },
  render: { node: ResourceListElement },
});
