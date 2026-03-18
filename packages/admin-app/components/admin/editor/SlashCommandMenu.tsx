"use client";

/**
 * SlashCommandMenu — Notion-style "/" command palette for inserting blocks.
 *
 * Triggered by typing "/" at the start of an empty paragraph.
 * Uses cmdk (via the Command UI component) for filtered search.
 * Positioned at cursor via @floating-ui/react.
 */

import React, { useState, useEffect, useCallback, useRef } from "react";
import { createPortal } from "react-dom";
import type { TElement } from "platejs";
import type { PlateEditor as PlateEditorType } from "platejs/react";
import { useFloating, offset, flip, shift, autoUpdate } from "@floating-ui/react";
import { getDOMSelectionBoundingClientRect } from "@platejs/floating";
import {
  Command,
  CommandInput,
  CommandList,
  CommandGroup,
  CommandItem,
  CommandEmpty,
} from "@/components/ui/command";
import { BLOCK_CATALOG, CATEGORY_LABELS, type BlockCatalogEntry } from "./block-catalog";

interface SlashCommandMenuProps {
  editor: PlateEditorType;
  open: boolean;
  onClose: () => void;
}

// Group entries by category
const groupedEntries = Object.entries(
  BLOCK_CATALOG.reduce<Record<string, BlockCatalogEntry[]>>((acc, entry) => {
    (acc[entry.category] ??= []).push(entry);
    return acc;
  }, {})
);

export function SlashCommandMenu({ editor, open, onClose }: SlashCommandMenuProps) {
  const [search, setSearch] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  // Position at cursor
  const { refs, floatingStyles } = useFloating({
    open,
    placement: "bottom-start",
    middleware: [offset(4), flip(), shift({ padding: 8 })],
    whileElementsMounted: autoUpdate,
  });

  // Update reference position to cursor on open
  useEffect(() => {
    if (open) {
      const rect = getDOMSelectionBoundingClientRect();
      refs.setReference({
        getBoundingClientRect: () => rect,
      });
      setSearch("");
      // Focus the search input after a tick
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open, refs]);

  const handleSelect = useCallback(
    (entry: BlockCatalogEntry) => {
      // Delete the "/" character from the current block
      const { selection } = editor;
      if (selection) {
        // Select back to delete the "/" trigger
        const point = selection.anchor;
        if (point.offset > 0) {
          editor.tf.delete({
            at: {
              anchor: { ...point, offset: point.offset - 1 },
              focus: point,
            },
          });
        }
      }

      // For text blocks: convert current block type
      const textBlocks = ["p", "h2", "h3", "h4", "h5", "h6", "blockquote", "todo", "toggle", "callout", "code_block"];
      if (textBlocks.includes(entry.key)) {
        editor.tf.toggleBlock(entry.key);
      } else {
        // For void/custom blocks: insert a new node
        const node: TElement = {
          type: entry.key,
          children: [{ text: "" }],
          ...(entry.defaultData || {}),
        };
        editor.tf.insertNodes(node);
      }

      onClose();
    },
    [editor, onClose]
  );

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div
      ref={refs.setFloating}
      style={{
        ...floatingStyles,
        zIndex: 50,
      }}
      className="slash-command-menu"
    >
      <Command shouldFilter={true} className="slash-command">
        <CommandInput
          ref={inputRef}
          placeholder="Search blocks..."
          value={search}
          onValueChange={setSearch}
          className="slash-command__input"
        />
        <CommandList className="slash-command__list">
          <CommandEmpty>No blocks found.</CommandEmpty>
          {groupedEntries.map(([category, entries]) => (
            <CommandGroup
              key={category}
              heading={CATEGORY_LABELS[category] || category}
            >
              {entries.map((entry) => (
                <CommandItem
                  key={entry.key}
                  value={`${entry.label} ${entry.description}`}
                  onSelect={() => handleSelect(entry)}
                  className="slash-command__item"
                >
                  <div>
                    <div className="slash-command__item-label">{entry.label}</div>
                    <div className="slash-command__item-desc">{entry.description}</div>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
          ))}
        </CommandList>
      </Command>
    </div>,
    document.body
  );
}
