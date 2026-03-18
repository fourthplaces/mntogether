"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Play } from "lucide-react";

export const AUDIO_A_KEY = "audio_a";

export function AudioAElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { title?: string; duration?: string; credit?: string };

  const updateData = useCallback(
    (field: string, value: string) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="audio-a" {...{contentEditable: false} as any}>
      <input className="void-input" placeholder="Audio title" value={data.title || ""} onChange={(e) => updateData("title", e.target.value)} style={{ fontStyle: "italic", fontSize: "0.95rem" }} />
      <div className="audio-a__player" style={{ opacity: 0.4, pointerEvents: "none", margin: "8px 0" }}>
        <Play size={16} strokeWidth={2} />
        <div style={{ flex: 1, height: "40px", background: "rgba(0,56,101,0.15)", borderRadius: "2px" }} />
        <input className="void-input" placeholder="0:00" value={data.duration || ""} onChange={(e) => updateData("duration", e.target.value)} style={{ width: "60px", fontSize: "0.75rem", textAlign: "right", pointerEvents: "auto" }} />
      </div>
      <input className="void-input" placeholder="Credit" value={data.credit || ""} onChange={(e) => updateData("credit", e.target.value)} style={{ fontSize: "0.72rem", color: "var(--pebble)" }} />
      {children}
    </PlateElement>
  );
}

export const AudioAPlugin = createPlatePlugin({
  key: AUDIO_A_KEY,
  node: { isElement: true, isVoid: true, type: AUDIO_A_KEY },
  render: { node: AudioAElement },
});
