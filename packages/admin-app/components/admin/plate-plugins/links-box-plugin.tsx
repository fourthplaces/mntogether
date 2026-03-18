"use client";

/**
 * LinksBoxPlugin — "See Also" reference box with titled links.
 *
 * Node type: "links_box"
 * Void: true (data stored as node attributes)
 *
 * Node data: { header, links: Array<{ title, url }> }
 */

import React, { useCallback } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import type { TElement } from "platejs";

export const LINKS_BOX_KEY = "links_box";

interface LinkItem {
  title: string;
  url: string;
}

interface LinksBoxData {
  header?: string;
  links?: LinkItem[];
}

export function LinksBoxElement(props: PlateElementProps) {
  const { children, editor, element, ...rest } = props;
  const data = element as TElement & LinksBoxData;
  const links = data.links || [];
  const header = data.header || "See Also";

  const updateLinks = useCallback(
    (newLinks: LinkItem[]) => {
      const path = editor.api.findPath(element);
      if (path) {
        editor.tf.setNodes({ links: newLinks } as Partial<TElement>, { at: path });
      }
    },
    [editor, element]
  );

  const updateHeader = useCallback(
    (value: string) => {
      const path = editor.api.findPath(element);
      if (path) {
        editor.tf.setNodes({ header: value } as Partial<TElement>, { at: path });
      }
    },
    [editor, element]
  );

  const addLink = () => updateLinks([...links, { title: "", url: "" }]);

  const removeLink = (index: number) => {
    updateLinks(links.filter((_, i) => i !== index));
  };

  const updateLink = (index: number, field: keyof LinkItem, value: string) => {
    const newLinks = links.map((link, i) =>
      i === index ? { ...link, [field]: value } : link
    );
    updateLinks(newLinks);
  };

  return (
    <PlateElement {...rest} element={element} editor={editor} className="links-a" contentEditable={false}>
      <input
        className="void-input links-a__header"
        value={header}
        onChange={(e) => updateHeader(e.target.value)}
        placeholder="Header (e.g. See Also)"
      />

      {links.map((link, i) => (
        <div key={i} className="links-a__item">
          <input
            className="void-input links-a__title"
            value={link.title}
            onChange={(e) => updateLink(i, "title", e.target.value)}
            placeholder="Link title"
          />
          <input
            className="void-input links-a__url"
            value={link.url}
            onChange={(e) => updateLink(i, "url", e.target.value)}
            placeholder="https://..."
          />
          <button
            type="button"
            className="block-action-btn"
            onClick={() => removeLink(i)}
            title="Remove link"
          >
            ×
          </button>
        </div>
      ))}

      <div className="block-actions">
        <button type="button" className="block-action-btn" onClick={addLink}>
          + Add Link
        </button>
      </div>

      {children}
    </PlateElement>
  );
}

export const LinksBoxPlugin = createPlatePlugin({
  key: LINKS_BOX_KEY,
  node: { isElement: true, isVoid: true, type: LINKS_BOX_KEY },
  render: { node: LinksBoxElement },
});
