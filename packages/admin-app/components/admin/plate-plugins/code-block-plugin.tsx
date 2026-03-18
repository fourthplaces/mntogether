"use client";

import React from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";

export const CODE_BLOCK_KEY = "code_block";

export function CodeBlockElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  return (
    <PlateElement {...rest} as="pre" className="code-block">
      <code>{children}</code>
    </PlateElement>
  );
}

export const CodeBlockPlugin = createPlatePlugin({
  key: CODE_BLOCK_KEY,
  node: { isElement: true, type: CODE_BLOCK_KEY },
  render: { node: CodeBlockElement },
});
