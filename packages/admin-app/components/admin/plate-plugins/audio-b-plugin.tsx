"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Play } from "lucide-react";

export const AUDIO_B_KEY = "audio_b";

export function AudioBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { title?: string; duration?: string; excerpt?: string };

  const updateData = useCallback(
    (field: string, value: string) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ [field]: value } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="audio-b" {...{contentEditable: false} as any}>
      <div>
        <Play size={14} strokeWidth={2} style={{ marginRight: "0.3em", flexShrink: 0 }} />
        <input className="void-input" placeholder="Excerpt or description..." value={data.excerpt || ""} onChange={(e) => updateData("excerpt", e.target.value)} style={{ display: "inline", width: "80%", fontStyle: "italic", color: "var(--slate)" }} />
      </div>
      <input className="void-input" placeholder="Audio title" value={data.title || ""} onChange={(e) => updateData("title", e.target.value)} style={{ fontSize: "0.75rem", color: "var(--slate)", marginTop: "8px" }} />
      <input className="void-input" placeholder="Duration (e.g. 3:42)" value={data.duration || ""} onChange={(e) => updateData("duration", e.target.value)} style={{ fontSize: "0.72rem", color: "var(--pebble)", width: "80px" }} />
      {children}
    </PlateElement>
  );
}

export const AudioBPlugin = createPlatePlugin({
  key: AUDIO_B_KEY,
  node: { isElement: true, isVoid: true, type: AUDIO_B_KEY },
  render: { node: AudioBElement },
});
