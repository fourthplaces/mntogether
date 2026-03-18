"use client";

/**
 * PullQuotePlugin — custom Plate.js block element for editorial pull quotes.
 *
 * In the editor: renders as a full-width bordered block (no float).
 * On the web-app: the AstRenderer applies CSS float (40% width, right).
 *
 * Node type: "pull_quote"
 * Void: false (contains editable text children)
 */

import React from "react";
import type { PlateElementProps } from "platejs/react";
import { createPlatePlugin, PlateElement } from "platejs/react";

export const PULL_QUOTE_KEY = "pull_quote";

export function PullQuoteElement(props: PlateElementProps) {
  const { children, ...rest } = props;
  return (
    <PlateElement {...rest} as="blockquote" className="pull-quote">
      <span className="pull-quote__hint">Pull Quote</span>
      {children}
    </PlateElement>
  );
}

export const PullQuotePlugin = createPlatePlugin({
  key: PULL_QUOTE_KEY,
  node: { isElement: true, type: PULL_QUOTE_KEY },
  render: { node: PullQuoteElement },
});
