"use client";

import Link from "next/link";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
  getWeeksOld,
  getStalenessLevel,
  getStalenessLabel,
  STALENESS_BORDER,
  STALENESS_TEXT,
} from "@/lib/staleness";

export interface EditionCardData {
  id: string;
  countyName: string;
  periodStart: string;
  periodEnd: string;
  status: string;
  filledSlots: number;
  totalSlots: number;
}

interface EditionKanbanCardProps extends EditionCardData {
  isDragOverlay?: boolean;
}

export function EditionKanbanCard({
  id,
  countyName,
  periodEnd,
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

  const weeksOld = getWeeksOld(periodEnd);
  const level = getStalenessLevel(weeksOld);
  const label = getStalenessLabel(weeksOld);
  const borderClass = STALENESS_BORDER[level];
  const textClass = STALENESS_TEXT[level];

  const card = (
    <div
      ref={isDragOverlay ? undefined : setNodeRef}
      style={isDragOverlay ? undefined : style}
      className={`bg-surface-raised border border-border border-l-[3px] ${borderClass} rounded-lg px-3 py-2 shadow-sm hover:shadow-card-hover transition-shadow group ${
        isDragOverlay ? "shadow-dialog rotate-1 scale-[1.02]" : ""
      }`}
      {...(isDragOverlay ? {} : attributes)}
    >
      <div className="flex items-start gap-2">
        {/* Drag handle */}
        <button
          className="mt-0.5 cursor-grab active:cursor-grabbing text-text-faint hover:text-text-muted shrink-0"
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

        <div className="flex-1 min-w-0">
          {/* County name */}
          <p className="text-sm font-medium text-text-primary truncate">
            {countyName}
          </p>

          {/* Staleness label */}
          <div className="flex items-center gap-2 mt-1">
            <span className={`text-[10px] font-medium ${textClass}`}>
              {level === "alert" && (
                <svg className="w-3 h-3 inline-block mr-0.5 -mt-px" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4.5c-.77-.833-2.694-.833-3.464 0L3.34 16.5c-.77.833.192 2.5 1.732 2.5z" />
                </svg>
              )}
              {label}
            </span>
          </div>
        </div>

        {/* Edit link (visible on hover) */}
        <Link
          href={`/admin/editions/${id}`}
          className="opacity-0 group-hover:opacity-100 text-text-muted hover:text-text-primary text-xs transition-opacity mt-0.5 shrink-0"
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
