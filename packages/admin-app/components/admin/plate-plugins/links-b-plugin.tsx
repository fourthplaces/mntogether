"use client";

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";
import { Plus, X } from "lucide-react";

export const LINKS_B_KEY = "links_b";

interface LinkItem { title: string; url: string; }

export function LinksBElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & { links?: LinkItem[] };
  const links = data.links || [];

  const updateLinks = useCallback(
    (newLinks: LinkItem[]) => {
      const path = editor.api.findPath(element);
      if (path) editor.tf.setNodes({ links: newLinks } as Partial<TElement>, { at: path });
    },
    [editor, element]
  );

  return (
    <PlateElement {...rest} element={element} editor={editor} className="links-b">
      <div contentEditable={false} onMouseDown={(e) => { if (!(e.target instanceof HTMLInputElement || e.target instanceof HTMLButtonElement)) e.preventDefault(); }}>
        {links.map((link, i) => (
          <div key={i} className="links-b__item" style={{ display: "flex", gap: "8px", padding: "8px 0" }}>
            <span className="links-b__num" style={{ fontFamily: "var(--font-display)", fontWeight: 500, fontSize: "0.95rem", color: "var(--pebble)", minWidth: "1.2em" }}>{i + 1}</span>
            <div style={{ flex: 1 }}>
              <input className="void-input links-b__title" value={link.title} onChange={(e) => { const l = [...links]; l[i] = { ...l[i], title: e.target.value }; updateLinks(l); }} placeholder="Link title" style={{ fontStyle: "italic", fontSize: "0.9rem" }} />
              <input className="void-input" value={link.url} onChange={(e) => { const l = [...links]; l[i] = { ...l[i], url: e.target.value }; updateLinks(l); }} placeholder="https://..." style={{ fontSize: "0.72rem", color: "var(--pebble)" }} />
            </div>
            <button type="button" className="block-action-btn" onClick={() => updateLinks(links.filter((_, j) => j !== i))}><X size={12} strokeWidth={2} /></button>
          </div>
        ))}
        <div className="block-actions">
          <button type="button" className="block-action-btn" onClick={() => updateLinks([...links, { title: "", url: "" }])}><Plus size={12} strokeWidth={2} /> Link</button>
        </div>
      </div>
      {children}
    </PlateElement>
  );
}

export const LinksBPlugin = createPlatePlugin({
  key: LINKS_B_KEY,
  node: { isElement: true, isVoid: true, type: LINKS_B_KEY },
  render: { node: LinksBElement },
});
