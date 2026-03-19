"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Plus } from "lucide-react";

export const KICKER_B_KEY = "kicker_b";

export function KickerBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { primary?: string; secondary?: string[]; color?: string };
  const secondary = data.secondary || [];

  const updateData = useCallback(
    (field: string, value: unknown) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="kicker-b">
      <div className="kicker-b__primary" style={{ borderTopColor: data.color || "var(--deep-forest)" }}>
        <input className="void-input" value={data.primary || ""} onChange={(e) => updateData("primary", e.target.value)} placeholder="Primary tag" style={{ fontSize: "0.88rem" }} />
      </div>
      <div className="kicker-b__secondary" style={{ display: "flex", gap: "6px", marginTop: "8px", flexWrap: "wrap" }}>
        {secondary.map((tag, i) => (
          <input key={i} className="void-input kicker-b__pill" value={tag} onChange={(e) => { const s = [...secondary]; s[i] = e.target.value; updateData("secondary", s); }} placeholder="Tag" style={{ width: "auto", minWidth: "60px", fontSize: "0.75rem", padding: "3px 10px", background: "rgba(0,0,0,0.04)" }} />
        ))}
      </div>
      <div className="block-actions">
        <button type="button" className="block-action-btn" onClick={() => updateData("secondary", [...secondary, ""])}><Plus size={12} strokeWidth={2} /> Pill</button>
      </div>
      {children}
    </PlateElement>
  );
}

export const KickerBPlugin = createPlatePlugin({
  key: KICKER_B_KEY,
  node: { isElement: true, isVoid: true, type: KICKER_B_KEY },
  render: { node: KickerBElement },
});
