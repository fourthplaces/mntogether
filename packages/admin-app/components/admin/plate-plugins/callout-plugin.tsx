"use client";

import React from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";
import { Lightbulb } from "lucide-react";

export const CALLOUT_KEY = "callout";

export function CalloutElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  return (
    <PlateElement {...rest} className="callout-block">
      <span {...{contentEditable: false} as any} className="callout-block__icon"><Lightbulb size={18} strokeWidth={2} /></span>
      <div className="callout-block__content">{children}</div>
    </PlateElement>
  );
}

export const CalloutPlugin = createPlatePlugin({
  key: CALLOUT_KEY,
  node: { isElement: true, type: CALLOUT_KEY },
  render: { node: CalloutElement },
});
