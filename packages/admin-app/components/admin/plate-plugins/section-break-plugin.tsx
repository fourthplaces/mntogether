"use client";

/**
 * SectionBreakPlugin — decorative section break (centered dots).
 *
 * Node type: "section_break"
 * Void: true (no editable content)
 */

import React from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";

export const SECTION_BREAK_KEY = "section_break";

export function SectionBreakElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  return (
    <PlateElement {...rest} className="section-break" {...{contentEditable: false} as any}>
      · · ·
      {children}
    </PlateElement>
  );
}

export const SectionBreakPlugin = createPlatePlugin({
  key: SECTION_BREAK_KEY,
  node: { isElement: true, isVoid: true, type: SECTION_BREAK_KEY },
  render: { node: SectionBreakElement },
});
