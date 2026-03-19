"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const ADDRESS_B_KEY = "address_b";

export function AddressBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { street?: string; city?: string; state?: string; zip?: string };

  const updateData = useCallback(
    (field: string, value: string) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="address-b">
      <div className="address-b__city" style={{ display: "flex", gap: "4px" }}>
        <input className="void-input" value={data.city || ""} onChange={(e) => updateData("city", e.target.value)} placeholder="City" style={{ fontFamily: "var(--font-condensed)", fontWeight: 500, textTransform: "uppercase" }} />
        <input className="void-input" value={data.state || ""} onChange={(e) => updateData("state", e.target.value)} placeholder="ST" style={{ width: "40px", fontFamily: "var(--font-condensed)", fontWeight: 500, textTransform: "uppercase" }} />
      </div>
      <div style={{ display: "flex", gap: "8px" }}>
        <input className="void-input address-b__street" value={data.street || ""} onChange={(e) => updateData("street", e.target.value)} placeholder="Street" style={{ color: "var(--slate)" }} />
        <input className="void-input address-b__zip" value={data.zip || ""} onChange={(e) => updateData("zip", e.target.value)} placeholder="ZIP" style={{ width: "60px", fontSize: "0.75rem", color: "var(--slate)" }} />
      </div>
      {children}
    </PlateElement>
  );
}

export const AddressBPlugin = createPlatePlugin({
  key: ADDRESS_B_KEY,
  node: { isElement: true, isVoid: true, type: ADDRESS_B_KEY },
  render: { node: AddressBElement },
});
