"use client";

import Link from "next/link";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

export interface EditionCardData {
  id: string;
  countyName: string;
  periodStart: string;
  periodEnd: string;
  status: string;
  rowCount: number;
}

interface EditionKanbanCardProps extends EditionCardData {
  isDragOverlay?: boolean;
}

export function EditionKanbanCard({
  id,
  countyName,
  rowCount,
  isDragOverlay,
}: EditionKanbanCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id, data: { type: "edition-card" } });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
  };

  const card = (
    <div
      ref={isDragOverlay ? undefined : setNodeRef}
      style={isDragOverlay ? undefined : style}
      className={`bg-surface-raised border border-border rounded-lg px-3 py-2 shadow-sm hover:shadow-card-hover transition-shadow group ${
        isDragOverlay ? "shadow-dialog rotate-1 scale-[1.02]" : ""
      }`}
      {...(isDragOverlay ? {} : attributes)}
    >
      <div className="flex items-center gap-2">
        {/* Drag handle */}
        <button
          className="cursor-grab active:cursor-grabbing text-text-faint hover:text-text-muted shrink-0"
          {...(isDragOverlay ? {} : listeners)}
          tabIndex={-1}
        >
          <svg className="w-3.5 h-3.5" viewBox="0 0 16 16" fill="currentColor">
            <circle cx="5" cy="3" r="1.5" />
            <circle cx="11" cy="3" r="1.5" />
            <circle cx="5" cy="8" r="1.5" />
            <circle cx="11" cy="8" r="1.5" />
            <circle cx="5" cy="13" r="1.5" />
            <circle cx="11" cy="13" r="1.5" />
          </svg>
        </button>

        {/* County name */}
        <p className="text-sm font-medium text-text-primary truncate flex-1 min-w-0">
          {countyName}
        </p>

        {/* Row count */}
        <span className="text-[10px] text-text-faint shrink-0">
          {rowCount} {rowCount === 1 ? "row" : "rows"}
        </span>

        {/* Edit link (visible on hover) */}
        <Link
          href={`/admin/editions/${id}`}
          className="opacity-0 group-hover:opacity-100 text-text-muted hover:text-text-primary text-xs transition-opacity shrink-0"
          onClick={(e) => e.stopPropagation()}
        >
          Edit
        </Link>
      </div>
    </div>
  );

  return card;
}

export function EditionKanbanCardOverlay(
  props: Omit<EditionKanbanCardProps, "isDragOverlay">
) {
  return <EditionKanbanCard {...props} isDragOverlay />;
}
