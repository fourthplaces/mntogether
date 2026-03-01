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
  filledSlots: number;
  totalSlots: number;
}

interface EditionKanbanCardProps extends EditionCardData {
  isDragOverlay?: boolean;
}

const STATUS_COLORS: Record<string, string> = {
  draft: "border-l-amber-400",
  in_review: "border-l-blue-400",
  approved: "border-l-emerald-400",
  published: "border-l-green-600",
};

function formatPeriod(start: string, end: string): string {
  const s = new Date(start + "T00:00:00");
  const e = new Date(end + "T00:00:00");
  const sMonth = s.toLocaleDateString("en-US", { month: "short" });
  const eMonth = e.toLocaleDateString("en-US", { month: "short" });
  if (sMonth === eMonth) {
    return `${sMonth} ${s.getDate()}–${e.getDate()}`;
  }
  return `${sMonth} ${s.getDate()} – ${eMonth} ${e.getDate()}`;
}

export function EditionKanbanCard({
  id,
  countyName,
  periodStart,
  periodEnd,
  filledSlots,
  totalSlots,
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

  const fillPercent = totalSlots > 0 ? Math.round((filledSlots / totalSlots) * 100) : 0;
  const borderColor = STATUS_COLORS[isDragOverlay ? "" : ""] ?? "";

  const card = (
    <div
      ref={isDragOverlay ? undefined : setNodeRef}
      style={isDragOverlay ? undefined : style}
      className={`bg-surface-raised border border-border border-l-[3px] ${STATUS_COLORS[isDragOverlay ? "draft" : "draft"]} rounded-lg px-3 py-2 shadow-sm hover:shadow-card-hover transition-shadow group ${
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

          {/* Period + fill rate */}
          <div className="flex items-center gap-2 mt-1">
            <span className="text-[10px] text-text-faint">
              {formatPeriod(periodStart, periodEnd)}
            </span>
            <span className="text-[10px] text-text-muted ml-auto">
              {filledSlots}/{totalSlots}
            </span>
          </div>

          {/* Fill bar */}
          {totalSlots > 0 && (
            <div className="mt-1.5 h-1 bg-stone-200 rounded-full overflow-hidden">
              <div
                className={`h-full rounded-full transition-all ${
                  fillPercent === 100
                    ? "bg-emerald-500"
                    : fillPercent >= 50
                      ? "bg-amber-500"
                      : "bg-stone-400"
                }`}
                style={{ width: `${fillPercent}%` }}
              />
            </div>
          )}
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
