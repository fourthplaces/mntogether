"use client";

/**
 * PlateEditor — WYSIWYG editor for post body content, backed by Plate.js.
 *
 * Supports: paragraphs, headings (h2-h4), bold, italic, underline,
 * blockquotes, ordered/unordered lists, links.
 *
 * Markdown round-tripping: imports from markdown on load, exports to
 * markdown on change via @platejs/markdown. The onChange callback fires
 * with markdown string — same interface as the old textarea.
 */

import { useCallback, useMemo, useRef, useEffect } from "react";
import type { Value } from "platejs";
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

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PlateEditorProps {
  /** Initial markdown content to load into the editor */
  initialMarkdown?: string;
  /** Called with markdown string on every content change */
  onChange?: (markdown: string) => void;
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
        e.preventDefault(); // prevent editor blur
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
    <div className="flex items-center gap-0.5 px-2 py-1.5 border-b border-border bg-surface-raised/50">
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
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function PlateEditor({
  initialMarkdown = "",
  onChange,
  placeholder = "Write your story...",
  disabled = false,
}: PlateEditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  // Create editor with plugins
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
    ],
  });

  // Deserialize initial markdown into editor on mount
  const initialized = useRef(false);
  useEffect(() => {
    if (!initialized.current && initialMarkdown) {
      initialized.current = true;
      try {
        const value = editor.getApi(MarkdownPlugin).markdown.deserialize(initialMarkdown);
        if (value && value.length > 0) {
          editor.tf.setValue(value);
        }
      } catch (e) {
        console.warn("Failed to deserialize markdown:", e);
      }
    }
  }, [editor, initialMarkdown]);

  // Serialize to markdown on change
  const handleChange = useCallback(
    ({ value }: { value: Value }) => {
      if (!onChangeRef.current) return;
      try {
        const md = editor.getApi(MarkdownPlugin).markdown.serialize();
        onChangeRef.current(md);
      } catch {
        // Serialization may fail during rapid typing — ignore
      }
    },
    [editor]
  );

  return (
    <div className="border border-border rounded-md overflow-hidden bg-white">
      <EditorToolbar editor={editor} />
      <Plate editor={editor} onChange={handleChange}>
        <PlateContent
          placeholder={placeholder}
          disabled={disabled}
          className="px-4 py-3 min-h-[300px] focus:outline-none prose prose-sm max-w-none"
          style={{
            fontFamily: "Georgia, serif",
            fontSize: "1rem",
            lineHeight: "1.7",
          }}
        />
      </Plate>
    </div>
  );
}
