"use client";

/**
 * PlateEditor — Notion-style block editor with broadsheet styling.
 *
 * Features:
 * - Floating toolbar on text selection (bold, italic, underline, link, code, strikethrough)
 * - Slash command menu (type "/" to insert any block type)
 * - Block-level + and ⋮⋮ handles on hover (via aboveNodes wrapper)
 * - Full broadsheet prototype component library as insertable blocks
 * - Real FeatureDeck/FeatureText fonts
 * - JSON AST storage (body_ast)
 */

import { useCallback, useRef, useEffect, useState } from "react";
import type { Value, TElement } from "platejs";
import { Plate, PlateContent, usePlateEditor, createPlatePlugin } from "platejs/react";
import type { PlateElementProps } from "platejs/react";
import {
  BoldPlugin,
  ItalicPlugin,
  UnderlinePlugin,
  StrikethroughPlugin,
  CodePlugin,
  BlockquotePlugin,
  H2Plugin,
  H3Plugin,
  H4Plugin,
  H5Plugin,
  H6Plugin,
} from "@platejs/basic-nodes/react";
import { LinkPlugin } from "@platejs/link/react";
import { ListPlugin } from "@platejs/list/react";
import { MarkdownPlugin } from "@platejs/markdown";

// Custom block plugins
import {
  PullQuotePlugin,
  SectionBreakPlugin,
  PhotoBlockPlugin,
  LinksBoxPlugin,
  ResourceListPlugin,
  PhotoAPlugin,
  PhotoBPlugin,
  AudioAPlugin,
  AudioBPlugin,
  KickerAPlugin,
  KickerBPlugin,
  ArticleMetaPlugin,
  LinksBPlugin,
  ListAPlugin,
  ListBPlugin,
  ResourceListBPlugin,
  AddressAPlugin,
  AddressBPlugin,
  PhoneAPlugin,
  PhoneBPlugin,
  TodoPlugin,
  TogglePlugin,
  CalloutPlugin,
  CodeBlockPlugin,
} from "./plate-plugins";

// Editor UI components
import { FloatingToolbar } from "./editor/FloatingToolbar";
import { SlashCommandMenu } from "./editor/SlashCommandMenu";
import { BlockHandles } from "./editor/BlockWrapper";

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
// Block handles plugin — wraps every element with + and ⋮⋮ handles
// ---------------------------------------------------------------------------

const BlockHandlesPlugin = createPlatePlugin({
  key: "block_handles",
  render: {
    aboveNodes:
      (props: PlateElementProps) => {
        const { children, editor, element } = props;

        const handleDelete = () => {
          const path = editor.api.findPath(element);
          if (path) {
            editor.tf.deleteNodes({ at: path });
          }
        };

        return function BlockHandlesWrapper({ children: innerChildren }: { children: React.ReactNode }) {
          return (
            <BlockHandles onDelete={handleDelete}>
              {innerChildren}
            </BlockHandles>
          );
        };
      },
  },
});

// ---------------------------------------------------------------------------
// All plugins
// ---------------------------------------------------------------------------

const ALL_PLUGINS = [
  // Marks
  BoldPlugin,
  ItalicPlugin,
  UnderlinePlugin,
  StrikethroughPlugin,
  CodePlugin,
  // Standard blocks
  BlockquotePlugin,
  H2Plugin,
  H3Plugin,
  H4Plugin,
  H5Plugin,
  H6Plugin,
  // Inline
  LinkPlugin,
  ListPlugin,
  // Markdown (for fallback deserialization)
  MarkdownPlugin,
  // Custom blocks — editorial
  PullQuotePlugin,
  SectionBreakPlugin,
  PhotoBlockPlugin,
  LinksBoxPlugin,
  ResourceListPlugin,
  // Custom blocks — media
  PhotoAPlugin,
  PhotoBPlugin,
  AudioAPlugin,
  AudioBPlugin,
  // Custom blocks — structure
  KickerAPlugin,
  KickerBPlugin,
  ArticleMetaPlugin,
  // Custom blocks — data
  LinksBPlugin,
  ListAPlugin,
  ListBPlugin,
  ResourceListBPlugin,
  AddressAPlugin,
  AddressBPlugin,
  PhoneAPlugin,
  PhoneBPlugin,
  // Notion-style blocks
  TodoPlugin,
  TogglePlugin,
  CalloutPlugin,
  CodeBlockPlugin,
  // UI infrastructure
  BlockHandlesPlugin,
];

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function PlateEditor({
  initialValue,
  initialMarkdown = "",
  onChange,
  placeholder = "Type / for commands...",
  disabled = false,
}: PlateEditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const [slashMenuOpen, setSlashMenuOpen] = useState(false);

  // Create editor with all plugins
  const editor = usePlateEditor({
    plugins: ALL_PLUGINS,
  });

  // Initialize editor on mount: prefer JSON AST, fall back to markdown
  const initialized = useRef(false);
  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    if (initialValue && Array.isArray(initialValue) && initialValue.length > 0) {
      editor.tf.setValue(initialValue);
    } else if (initialMarkdown) {
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

  // Handle "/" key to open slash command menu
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "/" && !slashMenuOpen) {
        const { selection } = editor;
        if (selection) {
          // Open slash menu in any empty paragraph
          const [node] = editor.api.nodes({ match: { type: "p" }, at: selection });
          if (node) {
            const [element] = node;
            const text = (element as TElement).children
              ?.map((c: { text?: string }) => c.text || "")
              .join("") || "";
            if (text === "" || text === "/") {
              setTimeout(() => setSlashMenuOpen(true), 0);
            }
          }
        }
      }
    },
    [editor, slashMenuOpen]
  );

  return (
    <Plate editor={editor} onChange={handleChange}>
      <PlateContent
        placeholder={placeholder}
        disabled={disabled}
        className="body-a focus:outline-none"
        onKeyDown={handleKeyDown}
      />
      <FloatingToolbar editor={editor} />
      <SlashCommandMenu
        editor={editor}
        open={slashMenuOpen}
        onClose={() => setSlashMenuOpen(false)}
      />
    </Plate>
  );
}
