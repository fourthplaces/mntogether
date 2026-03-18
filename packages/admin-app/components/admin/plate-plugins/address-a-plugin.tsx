"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const ADDRESS_A_KEY = "address_a";

export function AddressAElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { street?: string; city?: string; state?: string; zip?: string; directionsUrl?: string };

  const updateData = useCallback(
    (field: string, value: string) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="address-a" contentEditable={false}>
      <div className="address-a__street">
        <input className="void-input" value={data.street || ""} onChange={(e) => updateData("street", e.target.value)} placeholder="Street address" style={{ fontSize: "0.95rem" }} />
      </div>
      <div className="address-a__city-state" style={{ display: "flex", gap: "4px", marginTop: "4px" }}>
        <input className="void-input" value={data.city || ""} onChange={(e) => updateData("city", e.target.value)} placeholder="City" style={{ fontSize: "0.75rem" }} />
        <input className="void-input" value={data.state || ""} onChange={(e) => updateData("state", e.target.value)} placeholder="ST" style={{ width: "40px", fontSize: "0.75rem" }} />
        <input className="void-input" value={data.zip || ""} onChange={(e) => updateData("zip", e.target.value)} placeholder="ZIP" style={{ width: "60px", fontSize: "0.75rem" }} />
      </div>
      <input className="void-input" value={data.directionsUrl || ""} onChange={(e) => updateData("directionsUrl", e.target.value)} placeholder="Directions URL (optional)" style={{ fontSize: "0.72rem", color: "var(--pebble)", marginTop: "8px" }} />
      {children}
    </PlateElement>
  );
}

export const AddressAPlugin = createPlatePlugin({
  key: ADDRESS_A_KEY,
  node: { isElement: true, isVoid: true, type: ADDRESS_A_KEY },
  render: { node: AddressAElement },
});
