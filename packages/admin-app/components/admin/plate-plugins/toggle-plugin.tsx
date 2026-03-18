"use client";

import React, { useState } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import { ChevronDown, ChevronRight } from "lucide-react";

export const TOGGLE_KEY = "toggle";

export function ToggleElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  const [open, setOpen] = useState(true);

  return (
    <PlateElement {...rest} className="toggle-block">
      <span
        {...{contentEditable: false} as any}
        className="toggle-block__trigger"
        onMouseDown={(e) => { e.preventDefault(); setOpen(!open); }}
      >
        {open ? <ChevronDown size={14} strokeWidth={2} /> : <ChevronRight size={14} strokeWidth={2} />}
      </span>
      <div className={`toggle-block__content ${open ? "" : "toggle-block__content--collapsed"}`}>
        {children}
      </div>
    </PlateElement>
  );
}

export const TogglePlugin = createPlatePlugin({
  key: TOGGLE_KEY,
  node: { isElement: true, type: TOGGLE_KEY },
  render: { node: ToggleElement },
});
