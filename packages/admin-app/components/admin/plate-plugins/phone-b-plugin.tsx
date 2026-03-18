"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const PHONE_B_KEY = "phone_b";

export function PhoneBElement(props: PlateElementProps) {
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
    <PlateElement {...rest} element={element} editor={editor} className="phone-b" {...{contentEditable: false} as any}>
      <input className="void-input phone-b__label" value={data.label || ""} onChange={(e) => updateData("label", e.target.value)} placeholder="Label (e.g. Telephone)" style={{ fontSize: "0.75rem", color: "var(--pebble)", letterSpacing: "0.06em", textTransform: "uppercase" }} />
      <input className="void-input phone-b__number" value={data.number || ""} onChange={(e) => updateData("number", e.target.value)} placeholder="Phone number" style={{ fontFamily: "var(--font-display)", fontWeight: 400, fontSize: "1.4rem" }} />
      <div className="phone-b__rule" style={{ borderTop: "1px solid rgba(0,0,0,0.06)", marginTop: "8px" }} />
      {children}
    </PlateElement>
  );
}

export const PhoneBPlugin = createPlatePlugin({
  key: PHONE_B_KEY,
  node: { isElement: true, isVoid: true, type: PHONE_B_KEY },
  render: { node: PhoneBElement },
});
