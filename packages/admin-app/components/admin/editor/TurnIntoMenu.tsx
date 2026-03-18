"use client";

/**
 * TurnIntoMenu — dropdown in the floating toolbar to convert a block's type.
 *
 * Shows current block type and lets the user convert to any text block type.
 * Uses a simple CSS dropdown (no external library needed).
 */

import React, { useState, useRef, useEffect } from "react";
import { usePlateEditor } from "platejs/react";
import { ChevronDown } from "lucide-react";
type PlateEditorType = NonNullable<ReturnType<typeof usePlateEditor>>;

const TURN_INTO_OPTIONS = [
  { type: "p", label: "Paragraph" },
  { type: "h2", label: "Heading 2" },
  { type: "h3", label: "Heading 3" },
  { type: "h4", label: "Heading 4" },
  { type: "h5", label: "Heading 5" },
  { type: "h6", label: "Heading 6" },
  { type: "blockquote", label: "Blockquote" },
  { type: "todo", label: "Todo" },
  { type: "toggle", label: "Toggle" },
  { type: "callout", label: "Callout" },
  { type: "code_block", label: "Code Block" },
] as const;

const TYPE_LABELS: Record<string, string> = Object.fromEntries(
  TURN_INTO_OPTIONS.map((o) => [o.type, o.label])
);

interface TurnIntoMenuProps {
  editor: PlateEditorType;
}

function getCurrentBlockType(editor: PlateEditorType): string {
  try {
    const { selection } = editor;
    if (!selection) return "p";
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const entries = Array.from(
      (editor.api as any).nodes({
        at: selection,
        match: (n: { type?: string }) => !!n.type,
        mode: "lowest",
      })
    );
    if (entries.length > 0) {
      const entry = entries[0] as [{ type?: string }, unknown];
      return entry[0].type || "p";
    }
  } catch {
    // fallback
  }
  return "p";
}

export function TurnIntoMenu({ editor }: TurnIntoMenuProps) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  const currentType = getCurrentBlockType(editor);
  const currentLabel = TYPE_LABELS[currentType] || "Paragraph";

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const handleSelect = (type: string) => {
    // First reset to paragraph, then set the new type (unless it's already p)
    if (currentType !== "p" && currentType !== type) {
      editor.tf.toggleBlock(currentType); // remove current
    }
    if (type !== "p") {
      editor.tf.toggleBlock(type); // apply new
    }
    setOpen(false);
  };

  return (
    <div className="turn-into-menu" ref={menuRef}>
      <button
        type="button"
        className="floating-toolbar__btn turn-into-menu__trigger"
        title="Turn into..."
        onMouseDown={(e) => {
          e.preventDefault();
          e.stopPropagation();
          setOpen(!open);
        }}
      >
        <span className="turn-into-menu__label">{currentLabel}</span>
        <ChevronDown size={12} strokeWidth={2} className="turn-into-menu__chevron" />
      </button>
      {open && (
        <div className="turn-into-menu__dropdown">
          {TURN_INTO_OPTIONS.map((opt) => (
            <button
              key={opt.type}
              type="button"
              className={`turn-into-menu__option ${opt.type === currentType ? "turn-into-menu__option--active" : ""}`}
              onMouseDown={(e) => {
                e.preventDefault();
                e.stopPropagation();
                handleSelect(opt.type);
              }}
            >
              {opt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
