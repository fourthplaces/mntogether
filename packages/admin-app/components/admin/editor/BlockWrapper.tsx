"use client";

/**
 * BlockHandles — rendered above each block element via Plate's aboveNodes.
 *
 * Shows + (insert) and ⋮⋮ (drag handle placeholder) on hover.
 * The + button opens the slash command menu callback.
 * Drag-and-drop will be wired in a future phase.
 */

import React from "react";

interface BlockHandlesProps {
  onInsert?: () => void;
  onDelete?: () => void;
  children: React.ReactNode;
}

export function BlockHandles({ onInsert, onDelete, children }: BlockHandlesProps) {
  return (
    <div className="block-wrapper">
      <div className="block-handle-gutter" contentEditable={false}>
        <button
          type="button"
          className="block-handle block-handle--insert"
          title="Insert block"
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onInsert?.();
          }}
        >
          +
        </button>
        <button
          type="button"
          className="block-handle block-handle--drag"
          title="Drag to reorder"
          onMouseDown={(e) => {
            e.preventDefault();
            // DnD will be wired in a future phase
          }}
        >
          ⋮⋮
        </button>
        {onDelete && (
          <button
            type="button"
            className="block-handle block-handle--delete"
            title="Delete block"
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onDelete();
            }}
          >
            ×
          </button>
        )}
      </div>
      {children}
    </div>
  );
}
