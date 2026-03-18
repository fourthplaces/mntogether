"use client";

/**
 * FloatingToolbar — appears above text selection for inline formatting.
 *
 * Replaces the static Word-style toolbar. Shows bold/italic/underline/link
 * buttons as a dark pill floating above the current selection.
 *
 * Uses @platejs/floating hooks for positioning.
 */

import React from "react";
import {
  useFloatingToolbarState,
  useFloatingToolbar,
  getDOMSelectionBoundingClientRect,
} from "@platejs/floating";
import { useEditorId, usePlateEditor } from "platejs/react";
import { Code, Link } from "lucide-react";
import { TurnIntoMenu } from "./TurnIntoMenu";

interface FloatingToolbarProps {
  editor: NonNullable<ReturnType<typeof usePlateEditor>>;
}

function MarkButton({
  editor,
  mark,
  label,
  children,
}: {
  editor: NonNullable<ReturnType<typeof usePlateEditor>>;
  mark: string;
  label: string;
  children: React.ReactNode;
}) {
  const isActive = (() => {
    try {
      return editor.api.isMarkActive(mark);
    } catch {
      return false;
    }
  })();

  return (
    <button
      type="button"
      title={label}
      onMouseDown={(e) => {
        e.preventDefault();
        editor.tf.toggleMark(mark);
      }}
      className={`floating-toolbar__btn ${isActive ? "floating-toolbar__btn--active" : ""}`}
    >
      {children}
    </button>
  );
}

export function FloatingToolbar({ editor }: FloatingToolbarProps) {
  const editorId = useEditorId();

  const state = useFloatingToolbarState({
    editorId,
    focusedEditorId: editorId, // single editor — always focused
    floatingOptions: {
      getBoundingClientRect: getDOMSelectionBoundingClientRect,
    },
  });

  const { ref, props, hidden } = useFloatingToolbar(state);

  if (hidden) return null;

  return (
    <div
      ref={ref}
      className="floating-toolbar"
      {...props}
    >
      <TurnIntoMenu editor={editor} />
      <span className="floating-toolbar__sep" />
      <MarkButton editor={editor} mark="bold" label="Bold (⌘B)">
        <strong>B</strong>
      </MarkButton>
      <MarkButton editor={editor} mark="italic" label="Italic (⌘I)">
        <em>I</em>
      </MarkButton>
      <MarkButton editor={editor} mark="underline" label="Underline (⌘U)">
        <u>U</u>
      </MarkButton>
      <MarkButton editor={editor} mark="strikethrough" label="Strikethrough">
        <s>S</s>
      </MarkButton>
      <MarkButton editor={editor} mark="code" label="Code">
        <Code size={14} strokeWidth={2} />
      </MarkButton>

      <span className="floating-toolbar__sep" />

      {/* Link button */}
      <button
        type="button"
        title="Link (⌘K)"
        onMouseDown={(e) => {
          e.preventDefault();
          const url = window.prompt("Enter URL:");
          if (url) {
            editor.tf.insertLink({ url });
          }
        }}
        className="floating-toolbar__btn"
      >
        <Link size={14} strokeWidth={2} />
      </button>
    </div>
  );
}
