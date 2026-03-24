"use client";

import Link from "next/link";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

interface WorkflowCardProps {
  id: string;
  title: string;
  postType?: string | null;
  isUrgent?: boolean | null;
  createdAt: string;
  isDragOverlay?: boolean;
}

const TYPE_COLORS: Record<string, string> = {
  story: "bg-pathway-lavender text-text-body",
  notice: "bg-pathway-warm text-text-body",
  exchange: "bg-pathway-sage text-text-body",
  event: "bg-info-bg text-info-text",
  spotlight: "bg-warning-bg text-warning-text",
  reference: "bg-surface-muted text-text-secondary",
};

function formatShortDate(dateString: string): string {
  const d = new Date(dateString);
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

export function WorkflowCard({
  id,
  title,
  postType,
  isUrgent,
  createdAt,
  isDragOverlay,
}: WorkflowCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id, data: { type: "card" } });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.4 : 1,
  };

  const typeClass = TYPE_COLORS[postType ?? ""] ?? "bg-surface-muted text-text-secondary";

  const card = (
    <div
      ref={isDragOverlay ? undefined : setNodeRef}
      style={isDragOverlay ? undefined : style}
      className={`bg-surface-raised border border-border rounded-lg px-3 py-2.5 shadow-sm hover:shadow-card-hover transition-shadow group ${
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
          <svg className="w-4 h-4" viewBox="0 0 16 16" fill="currentColor">
            <circle cx="5" cy="3" r="1.5" />
            <circle cx="11" cy="3" r="1.5" />
            <circle cx="5" cy="8" r="1.5" />
            <circle cx="11" cy="8" r="1.5" />
            <circle cx="5" cy="13" r="1.5" />
            <circle cx="11" cy="13" r="1.5" />
          </svg>
        </button>

        <div className="flex-1 min-w-0">
          {/* Title */}
          <p className="text-sm font-medium text-text-primary truncate">
            {title}
          </p>

          {/* Meta row */}
          <div className="flex items-center gap-2 mt-1.5">
            {postType && (
              <span
                className={`text-[10px] font-medium px-1.5 py-0.5 rounded ${typeClass}`}
              >
                {postType}
              </span>
            )}
            {isUrgent && (
              <span className="text-[10px] text-danger-text font-semibold">Urgent</span>
            )}
            <span className="text-[10px] text-text-faint ml-auto">
              {formatShortDate(createdAt)}
            </span>
          </div>
        </div>

        {/* Edit link (visible on hover) */}
        <Link
          href={`/admin/posts/${id}`}
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

/** Simplified overlay version — used by DragOverlay. */
export function WorkflowCardOverlay(props: Omit<WorkflowCardProps, "isDragOverlay">) {
  return <WorkflowCard {...props} isDragOverlay />;
}
