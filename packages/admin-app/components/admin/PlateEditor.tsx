"use client";

/**
 * PlateEditor — Notion-style block editor with broadsheet styling.
 *
 * Features:
 * - Floating toolbar on text selection (bold, italic, underline, link, code, strikethrough)
 * - Slash command menu (type "/" to insert any block type)
 * - Block-level drag-and-drop reordering via @platejs/dnd
 * - Block handles: + (insert) and grip (drag) on hover
 * - Full broadsheet prototype component library as insertable blocks
 * - Real FeatureDeck/FeatureText fonts
 * - JSON AST storage (body_ast)
 */

import { useCallback, useRef, useState } from "react";
import type { Value, TElement, Path } from "platejs";
import { NodeIdPlugin } from "platejs";
import { Plate, PlateContent, usePlateEditor } from "platejs/react";
import type { PlateElementProps } from "platejs/react";
import { useMediaUpload } from "@/lib/hooks/useMediaUpload";
import { PHOTO_A_KEY } from "./plate-plugins/photo-a-plugin";
import { PhotoPickerProvider } from "./plate-plugins/photo-picker-context";
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
import { DndPlugin } from "@platejs/dnd";
import { DndProvider } from "react-dnd";
import { HTML5Backend } from "react-dnd-html5-backend";

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
import { BlockDraggable } from "./editor/BlockWrapper";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PlateEditorProps {
  /** Initial editor value (JSON AST). */
  initialValue?: Value | null;
  /** Called with JSON AST on every content change. */
  onChange?: (value: Value) => void;
  /** Placeholder text when editor is empty */
  placeholder?: string;
  /** Disable editing */
  disabled?: boolean;
}

// ---------------------------------------------------------------------------
// Shared ref: lets the BlockDraggable's + button trigger the slash menu
// ---------------------------------------------------------------------------

export const slashMenuRequestRef: { current: (() => void) | null } = { current: null };

// ---------------------------------------------------------------------------
// BlockDraggable render wrapper for DndPlugin.aboveNodes
//
// Plate's aboveNodes receives the element props and must return a
// component that wraps each block's children. We create a closure
// that captures the element/editor and passes callbacks to BlockDraggable.
// ---------------------------------------------------------------------------

function blockDraggableWrapper(props: PlateElementProps) {
  const { editor, element } = props;

  const handleDelete = () => {
    const path = editor.api.findPath(element);
    if (path) {
      editor.tf.removeNodes({ at: path });
    }
  };

  const handleInsert = () => {
    const path = editor.api.findPath(element);
    if (!path) return;
    const insertPath = [...path.slice(0, -1), path[path.length - 1] + 1] as Path;
    editor.tf.insertNodes(
      { type: "p", children: [{ text: "" }] } as TElement,
      { at: insertPath }
    );
    editor.tf.select({ path: [...insertPath, 0], offset: 0 });
    slashMenuRequestRef.current?.();
  };

  return function BlockDraggableWrapper({ children }: { children: React.ReactNode }) {
    return (
      <BlockDraggable
        element={element}
        editor={editor}
        onInsert={handleInsert}
        onDelete={handleDelete}
      >
        {children}
      </BlockDraggable>
    );
  };
}

// ---------------------------------------------------------------------------
// DnD plugin configuration
// ---------------------------------------------------------------------------

const ConfiguredDndPlugin = DndPlugin.configure({
  options: {
    enableScroller: true,
  },
  render: {
    aboveNodes: blockDraggableWrapper,
    aboveSlate: ({ children }) => (
      <DndProvider backend={HTML5Backend}>{children}</DndProvider>
    ),
  },
});

// ---------------------------------------------------------------------------
// All plugins
// ---------------------------------------------------------------------------

const ALL_PLUGINS = [
  // Node IDs — required for DnD (assigns unique id to each block element)
  NodeIdPlugin,
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
  // DnD — drag handles + block reordering
  ConfiguredDndPlugin,
];

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function PlateEditor({
  initialValue,
  onChange,
  placeholder = "Type / for commands...",
  disabled = false,
}: PlateEditorProps) {
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const [slashMenuOpen, setSlashMenuOpen] = useState(false);

  // Wire the shared ref so + button can open the slash menu
  slashMenuRequestRef.current = useCallback(() => {
    setTimeout(() => setSlashMenuOpen(true), 50);
  }, []);

  // Create editor with all plugins and initial value
  const editor = usePlateEditor({
    plugins: ALL_PLUGINS,
    value: initialValue && Array.isArray(initialValue) && initialValue.length > 0
      ? initialValue
      : undefined,
  });

  // Emit JSON AST on change
  const handleChange = useCallback(
    ({ value }: { value: Value }) => {
      if (!onChangeRef.current) return;
      onChangeRef.current(value);
    },
    []
  );

  // File-drop: drop an image onto the body editor to upload it to the Media
  // Library and insert a photo_a (full-width) node at the end of the
  // document. Dropping multiple images appends them in order. Non-image
  // files are ignored.
  //
  // We append rather than trying to place at the drop position because
  // mapping a DOM point back to a Slate path is fiddly and the block-level
  // DnD (already wired) lets editors drag the new block into place in one
  // motion. Predictable beats clever here.
  const { uploadFiles } = useMediaUpload();
  const handleDragOver = useCallback((e: React.DragEvent) => {
    if (e.dataTransfer?.types.includes("Files")) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    }
  }, []);
  const handleDrop = useCallback(
    async (e: React.DragEvent) => {
      const files = Array.from(e.dataTransfer?.files || []).filter((f) =>
        f.type.startsWith("image/"),
      );
      if (files.length === 0) return;
      // Only swallow the event if we're actually going to handle it — leaves
      // non-image drops to react-dnd / browser default behavior.
      e.preventDefault();
      e.stopPropagation();
      const uploaded = await uploadFiles(files);
      for (const m of uploaded) {
        if (!m) continue;
        editor.tf.insertNodes(
          {
            type: PHOTO_A_KEY,
            src: m.url,
            mediaId: m.id,
            caption: "",
            credit: "",
            children: [{ text: "" }],
          } as TElement,
          { at: [editor.children.length] },
        );
      }
    },
    [editor, uploadFiles],
  );

  // Handle "/" key to open slash command menu
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "/" && !slashMenuOpen) {
        const { selection } = editor;
        if (selection) {
          const [node] = editor.api.nodes({ match: { type: "p" }, at: selection });
          if (node) {
            const [element] = node;
            const text = (element as TElement).children
              ?.map((c) => (c as { text?: string }).text || "")
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
    <PhotoPickerProvider>
      <Plate editor={editor} onChange={handleChange}>
        <PlateContent
          placeholder={placeholder}
          disabled={disabled}
          className="body-a focus:outline-none"
          onKeyDown={handleKeyDown}
          onDragOver={handleDragOver}
          onDrop={handleDrop}
        />
        <FloatingToolbar editor={editor} />
        <SlashCommandMenu
          editor={editor}
          open={slashMenuOpen}
          onClose={() => setSlashMenuOpen(false)}
        />
      </Plate>
    </PhotoPickerProvider>
  );
}
