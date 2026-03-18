"use client";

/**
 * PlateEditor — Single-pane WYSIWYG editor for post body content.
 *
 * Renders content with broadsheet newspaper styling (body-a class).
 * Custom block types: pull quotes, section breaks, inline photos,
 * links boxes, resource lists.
 *
 * Accepts and emits Plate.js Value (JSON AST) — not markdown.
 * Fallback: can deserialize from markdown for existing posts that
 * don't yet have body_ast.
 */

import { useCallback, useRef, useEffect, useState } from "react";
import type { Value, TElement } from "platejs";
import { Plate, PlateContent, usePlateEditor } from "platejs/react";
import {
  BoldPlugin,
  ItalicPlugin,
  UnderlinePlugin,
  BlockquotePlugin,
  H2Plugin,
  H3Plugin,
  H4Plugin,
} from "@platejs/basic-nodes/react";
import { LinkPlugin } from "@platejs/link/react";
import { ListPlugin } from "@platejs/list/react";
import { MarkdownPlugin } from "@platejs/markdown";

import {
  PullQuotePlugin,
  PULL_QUOTE_KEY,
  SectionBreakPlugin,
  SECTION_BREAK_KEY,
  PhotoBlockPlugin,
  PHOTO_BLOCK_KEY,
  LinksBoxPlugin,
  LINKS_BOX_KEY,
  ResourceListPlugin,
  RESOURCE_LIST_KEY,
} from "./plate-plugins";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PlateEditorProps {
  /** Initial editor value (JSON AST). Takes precedence over initialMarkdown. */
  initialValue?: Value | null;
  /** Fallback: initial markdown to deserialize if initialValue is null. */
  initialMarkdown?: string;
  /** Called with JSON AST on every content change. */
  onChange?: (value: Value) => void;
  /** Placeholder text when editor is empty */
  placeholder?: string;
  /** Disable editing */
  disabled?: boolean;
}

// ---------------------------------------------------------------------------
// Toolbar button
// ---------------------------------------------------------------------------

function ToolbarButton({
  active,
  onClick,
  title,
  children,
}: {
  active?: boolean;
  onClick: () => void;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onMouseDown={(e) => {
        e.preventDefault();
        onClick();
      }}
      title={title}
      className={`px-2 py-1 rounded text-sm font-medium transition-colors ${
        active
          ? "bg-surface-muted text-text-primary"
          : "text-text-muted hover:text-text-primary hover:bg-surface-muted/50"
      }`}
    >
      {children}
    </button>
  );
}

// ---------------------------------------------------------------------------
// Insert menu
// ---------------------------------------------------------------------------

function InsertMenu({ editor }: { editor: ReturnType<typeof usePlateEditor> }) {
  const [open, setOpen] = useState(false);

  const insertBlock = (type: string, data?: Record<string, unknown>) => {
    const node: TElement = {
      type,
      children: [{ text: "" }],
      ...data,
    };
    editor.tf.insertNodes(node);
    setOpen(false);
  };

  return (
    <div className="relative inline-block">
      <ToolbarButton
        onClick={() => setOpen(!open)}
        title="Insert block"
      >
        + Insert
      </ToolbarButton>
      {open && (
        <div
          className="absolute top-full left-0 mt-1 bg-white border border-border rounded shadow-lg z-50 min-w-[180px] py-1"
          onMouseDown={(e) => e.preventDefault()}
        >
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-sm hover:bg-surface-muted/50"
            onClick={() => insertBlock(PULL_QUOTE_KEY)}
          >
            Pull Quote
          </button>
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-sm hover:bg-surface-muted/50"
            onClick={() => insertBlock(SECTION_BREAK_KEY)}
          >
            Section Break
          </button>
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-sm hover:bg-surface-muted/50"
            onClick={() => insertBlock(PHOTO_BLOCK_KEY, { src: "", caption: "", credit: "", variant: "c" })}
          >
            Inline Photo
          </button>
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-sm hover:bg-surface-muted/50"
            onClick={() => insertBlock(LINKS_BOX_KEY, { header: "See Also", links: [{ title: "", url: "" }] })}
          >
            Links Box
          </button>
          <button
            type="button"
            className="w-full text-left px-3 py-1.5 text-sm hover:bg-surface-muted/50"
            onClick={() => insertBlock(RESOURCE_LIST_KEY, { items: [{ name: "", detail: "" }] })}
          >
            Resource List
          </button>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

function EditorToolbar({ editor }: { editor: ReturnType<typeof usePlateEditor> }) {
  const isMarkActive = (type: string) => {
    try {
      return editor.api.isMarkActive(type);
    } catch {
      return false;
    }
  };

  const toggleMark = (type: string) => {
    editor.tf.toggleMark(type);
  };

  const isBlockActive = (type: string) => {
    try {
      const nodes = editor.api.nodes({ match: { type } });
      return !!nodes.next().value;
    } catch {
      return false;
    }
  };

  const toggleBlock = (type: string) => {
    editor.tf.toggleBlock(type);
  };

  return (
    <div className="flex items-center gap-0.5 px-3 py-1.5 border-b border-border bg-surface-raised/50 sticky top-0 z-10">
      <ToolbarButton
        active={isMarkActive("bold")}
        onClick={() => toggleMark("bold")}
        title="Bold (⌘B)"
      >
        <strong>B</strong>
      </ToolbarButton>
      <ToolbarButton
        active={isMarkActive("italic")}
        onClick={() => toggleMark("italic")}
        title="Italic (⌘I)"
      >
        <em>I</em>
      </ToolbarButton>
      <ToolbarButton
        active={isMarkActive("underline")}
        onClick={() => toggleMark("underline")}
        title="Underline (⌘U)"
      >
        <u>U</u>
      </ToolbarButton>

      <div className="w-px h-5 bg-border mx-1" />

      <ToolbarButton
        active={isBlockActive("h2")}
        onClick={() => toggleBlock("h2")}
        title="Heading 2"
      >
        H2
      </ToolbarButton>
      <ToolbarButton
        active={isBlockActive("h3")}
        onClick={() => toggleBlock("h3")}
        title="Heading 3"
      >
        H3
      </ToolbarButton>
      <ToolbarButton
        active={isBlockActive("h4")}
        onClick={() => toggleBlock("h4")}
        title="Heading 4"
      >
        H4
      </ToolbarButton>

      <div className="w-px h-5 bg-border mx-1" />

      <ToolbarButton
        active={isBlockActive("blockquote")}
        onClick={() => toggleBlock("blockquote")}
        title="Blockquote"
      >
        &ldquo;
      </ToolbarButton>

      <div className="w-px h-5 bg-border mx-1" />

      <InsertMenu editor={editor} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function PlateEditor({
  initialValue,
  initialMarkdown = "",
  onChange,
  placeholder = "Write your story...",
  disabled = false,
}: PlateEditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  // Create editor with all plugins
  const editor = usePlateEditor({
    plugins: [
      BoldPlugin,
      ItalicPlugin,
      UnderlinePlugin,
      BlockquotePlugin,
      H2Plugin,
      H3Plugin,
      H4Plugin,
      LinkPlugin,
      ListPlugin,
      MarkdownPlugin,
      // Custom block plugins
      PullQuotePlugin,
      SectionBreakPlugin,
      PhotoBlockPlugin,
      LinksBoxPlugin,
      ResourceListPlugin,
    ],
  });

  // Initialize editor on mount: prefer JSON AST, fall back to markdown
  const initialized = useRef(false);
  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    if (initialValue && Array.isArray(initialValue) && initialValue.length > 0) {
      // Load from JSON AST
      editor.tf.setValue(initialValue);
    } else if (initialMarkdown) {
      // Fall back to markdown deserialization
      try {
        const value = editor.getApi(MarkdownPlugin).markdown.deserialize(initialMarkdown);
        if (value && value.length > 0) {
          editor.tf.setValue(value);
        }
      } catch (e) {
        console.warn("Failed to deserialize markdown:", e);
      }
    }
  }, [editor, initialValue, initialMarkdown]);

  // Emit JSON AST on change
  const handleChange = useCallback(
    ({ value }: { value: Value }) => {
      if (!onChangeRef.current) return;
      onChangeRef.current(value);
    },
    []
  );

  return (
    <>
      <EditorToolbar editor={editor} />
      <Plate editor={editor} onChange={handleChange}>
        <PlateContent
          placeholder={placeholder}
          disabled={disabled}
          className="body-a focus:outline-none"
        />
      </Plate>
    </>
  );
}
