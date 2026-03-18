"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const PHONE_A_KEY = "phone_a";

export function PhoneAElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { number?: string; display?: string; label?: string };

  const updateData = useCallback(
    (field: string, value: string) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="phone-a" contentEditable={false}>
      <input className="void-input phone-a__number" value={data.number || ""} onChange={(e) => updateData("number", e.target.value)} placeholder="Phone number" style={{ fontSize: "0.85rem" }} />
      <input className="void-input" value={data.label || ""} onChange={(e) => updateData("label", e.target.value)} placeholder="Label (optional)" style={{ fontSize: "0.72rem", color: "var(--pebble)", marginTop: "4px" }} />
      {children}
    </PlateElement>
  );
}

export const PhoneAPlugin = createPlatePlugin({
  key: PHONE_A_KEY,
  node: { isElement: true, isVoid: true, type: PHONE_A_KEY },
  render: { node: PhoneAElement },
});
