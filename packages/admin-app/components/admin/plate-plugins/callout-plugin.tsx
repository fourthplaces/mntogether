"use client";

import React from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";

export const CALLOUT_KEY = "callout";

export function CalloutElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  return (
    <PlateElement {...rest} className="callout-block">
      <span contentEditable={false} className="callout-block__icon">💡</span>
      <div className="callout-block__content">{children}</div>
    </PlateElement>
  );
}

export const CalloutPlugin = createPlatePlugin({
  key: CALLOUT_KEY,
  node: { isElement: true, type: CALLOUT_KEY },
  render: { node: CalloutElement },
});
