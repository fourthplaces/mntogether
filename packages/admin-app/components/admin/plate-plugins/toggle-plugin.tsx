"use client";

import React, { useState } from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";

export const TOGGLE_KEY = "toggle";

export function ToggleElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  const [open, setOpen] = useState(true);

  return (
    <PlateElement {...rest} className="toggle-block">
      <span
        contentEditable={false}
        className="toggle-block__trigger"
        onMouseDown={(e) => { e.preventDefault(); setOpen(!open); }}
      >
        {open ? "▾" : "▸"}
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
