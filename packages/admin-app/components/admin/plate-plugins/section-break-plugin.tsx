"use client";

/**
 * SectionBreakPlugin — decorative section break (centered dots).
 *
 * Node type: "section_break"
 * Void: true (no editable content)
 *
 * The `· · ·` glyphs go inside a `contentEditable={false}` wrapper so
 * browsers don't put a text caret in them. Void elements still get
 * Slate's zero-width `{children}` rendered — that stays outside the
 * non-editable wrapper as usual.
 */

import React from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";

export const SECTION_BREAK_KEY = "section_break";

export function SectionBreakElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  return (
    <PlateElement {...rest} className="section-break">
      <span contentEditable={false} style={{ userSelect: "none" }}>
        · · ·
      </span>
      {children}
    </PlateElement>
  );
}

export const SectionBreakPlugin = createPlatePlugin({
  key: SECTION_BREAK_KEY,
  node: { isElement: true, isVoid: true, type: SECTION_BREAK_KEY },
  render: { node: SectionBreakElement },
});
