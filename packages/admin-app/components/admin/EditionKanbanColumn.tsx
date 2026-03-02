"use client";

import { useDroppable } from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import {
  EditionKanbanCard,
  type EditionCardData,
} from "./EditionKanbanCard";

interface EditionKanbanColumnProps {
  id: string;
  label: string;
  count: number;
  colorClass: string;
  editions: EditionCardData[];
}

export function EditionKanbanColumn({
  id,
  label,
  count,
  colorClass,
  editions,
}: EditionKanbanColumnProps) {
  const { setNodeRef, isOver } = useDroppable({ id });

  return (
    <div
      className={`flex flex-col rounded-xl ${colorClass} w-72 shrink-0 transition-all ${
        isOver ? "ring-2 ring-admin-accent ring-offset-2" : ""
      }`}
    >
      {/* Column header */}
      <div className="flex items-center justify-between px-4 py-3">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-semibold text-text-primary">{label}</h3>
          <span className="text-xs font-medium text-text-muted bg-surface-raised px-2 py-0.5 rounded-full">
            {count}
          </span>
        </div>
      </div>

      {/* Scrollable card list */}
      <div
        ref={setNodeRef}
        className="flex-1 overflow-y-auto px-3 pb-3 space-y-2 min-h-0"
      >
        <SortableContext
          items={editions.map((e) => e.id)}
          strategy={verticalListSortingStrategy}
        >
          {editions.map((edition) => (
            <EditionKanbanCard key={edition.id} {...edition} />
          ))}
        </SortableContext>

        {editions.length === 0 && (
          <div className="flex items-center justify-center h-24 text-xs text-text-faint">
            No editions
          </div>
        )}
      </div>
    </div>
  );
}
