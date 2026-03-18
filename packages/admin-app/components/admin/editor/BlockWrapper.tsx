"use client";

/**
 * BlockWrapper — wraps every block element with Notion-style chrome.
 *
 * On hover, reveals a left gutter with:
 *   + button → opens block picker to insert above
 *   ⋮⋮ button → drag handle for reordering (via @dnd-kit/sortable)
 */

import React, { useState, useCallback } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

interface BlockWrapperProps {
  /** Stable ID for this block (used by @dnd-kit) */
  id: string;
  /** Callback to open the block picker for inserting a block */
  onInsertAbove?: () => void;
  children: React.ReactNode;
}

export function BlockWrapper({ id, onInsertAbove, children }: BlockWrapperProps) {
  const [hovered, setHovered] = useState(false);

  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="block-wrapper"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      {...attributes}
    >
      {/* Left gutter with handles */}
      <div
        className="block-handle-gutter"
        style={{ opacity: hovered ? 1 : 0 }}
        contentEditable={false}
      >
        {/* Insert handle */}
        <button
          type="button"
          className="block-handle block-handle--insert"
          title="Insert block"
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onInsertAbove?.();
          }}
        >
          +
        </button>

        {/* Drag handle */}
        <button
          type="button"
          className="block-handle block-handle--drag"
          title="Drag to reorder"
          {...listeners}
        >
          ⋮⋮
        </button>
      </div>

      {/* The actual block content */}
      {children}
    </div>
  );
}
