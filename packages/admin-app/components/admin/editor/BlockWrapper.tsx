"use client";

/**
 * BlockDraggable — wraps each block element with drag-and-drop handles.
 *
 * Uses @platejs/dnd (react-dnd) for real drag-and-drop reordering.
 * Shows a gutter to the LEFT of the content column with:
 *   - Plus button to insert a new block (opens slash menu)
 *   - GripVertical drag handle for reordering
 *
 * Rendered via DndPlugin's `render.aboveNodes`.
 */

import React, { memo } from "react";
import { useDraggable, useDropLine } from "@platejs/dnd";
import type { TElement } from "platejs";
import { GripVertical, Plus, Trash2 } from "lucide-react";

// Keys that should NOT be draggable (structural elements)
const UNDRAGGABLE_KEYS = new Set(["column", "tr", "td", "th"]);

interface BlockDraggableProps {
  element: TElement;
  editor: any;
  onInsert?: () => void;
  onDelete?: () => void;
  children: React.ReactNode;
}

const MemoizedChildren = memo(
  ({ children }: { children: React.ReactNode }) => <>{children}</>,
  (prev, next) => prev.children === next.children
);
MemoizedChildren.displayName = "MemoizedChildren";

function DropLine({ id }: { id?: string }) {
  const { dropLine } = useDropLine({ id });
  if (!dropLine) return null;

  return (
    <div
      className={`block-dropline block-dropline--${dropLine}`}
      contentEditable={false}
    />
  );
}

export function BlockDraggable({
  element,
  editor,
  onInsert,
  onDelete,
  children,
}: BlockDraggableProps) {
  const id = (element as any).id as string | undefined;
  const type = (element as any).type as string | undefined;

  // Don't wrap undraggable structural elements
  if (type && UNDRAGGABLE_KEYS.has(type)) {
    return <>{children}</>;
  }

  // Skip if element has no ID (NodeIdPlugin hasn't assigned one yet)
  if (!id) {
    return (
      <div className="block-wrapper">
        <div
          className="block-handle-gutter"
          contentEditable={false}
          onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); }}
          style={{ userSelect: "none" }}
        >
          <button
            type="button"
            className="block-handle block-handle--insert"
            title="Insert block below"
            onMouseDown={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onInsert?.();
            }}
          >
            <Plus size={14} strokeWidth={2} />
          </button>
        </div>
        {children}
      </div>
    );
  }

  return (
    <DraggableBlock
      id={id}
      element={element}
      editor={editor}
      onInsert={onInsert}
      onDelete={onDelete}
    >
      {children}
    </DraggableBlock>
  );
}

/** Inner component that uses the DnD hooks (requires element to have an id). */
function DraggableBlock({
  id,
  element,
  editor,
  onInsert,
  onDelete,
  children,
}: {
  id: string;
  element: TElement;
  editor: any;
  onInsert?: () => void;
  onDelete?: () => void;
  children: React.ReactNode;
}) {
  const { isDragging, handleRef, nodeRef, previewRef } = useDraggable({
    element,
  });

  return (
    <div
      className={`block-wrapper${isDragging ? " block-wrapper--dragging" : ""}`}
      ref={nodeRef}
    >
      {/* Gutter: positioned in the left margin, outside the content column */}
      <div
        className="block-handle-gutter"
        contentEditable={false}
        onMouseDown={(e) => { e.preventDefault(); e.stopPropagation(); }}
        style={{ userSelect: "none" }}
      >
        <button
          type="button"
          className="block-handle block-handle--insert"
          title="Insert block below"
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onInsert?.();
          }}
        >
          <Plus size={14} strokeWidth={2} />
        </button>
        <button
          type="button"
          className="block-handle block-handle--drag"
          title="Drag to reorder"
          ref={handleRef}
          onMouseDown={(e) => { e.stopPropagation(); }}
        >
          <GripVertical size={14} strokeWidth={1.5} />
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
            <Trash2 size={12} strokeWidth={1.5} />
          </button>
        )}
      </div>

      {/* Hidden element for multi-block drag preview */}
      <div ref={previewRef} />

      {/* Actual block content + drop line indicator */}
      <MemoizedChildren>{children}</MemoizedChildren>
      <DropLine id={id} />
    </div>
  );
}
