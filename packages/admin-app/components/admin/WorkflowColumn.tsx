"use client";

import { useDroppable } from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { WorkflowCard } from "./WorkflowCard";

interface Post {
  id: string;
  title: string;
  postType?: string | null;
  isUrgent?: boolean | null;
  createdAt: string;
}

interface WorkflowColumnProps {
  id: string;
  label: string;
  count: number;
  colorClass: string;
  posts: Post[];
}

export function WorkflowColumn({
  id,
  label,
  count,
  colorClass,
  posts,
}: WorkflowColumnProps) {
  const { setNodeRef, isOver } = useDroppable({ id });

  return (
    <div
      className={`flex flex-col rounded-xl ${colorClass} min-h-[400px] transition-all ${
        isOver ? "ring-2 ring-admin-accent ring-offset-2" : ""
      }`}
    >
      {/* Column header */}
      <div className="flex items-center justify-between px-4 py-3">
        <h3 className="text-sm font-semibold text-text-primary">{label}</h3>
        <span className="text-xs font-medium text-text-muted bg-surface-raised px-2 py-0.5 rounded-full">
          {count}
        </span>
      </div>

      {/* Scrollable card list */}
      <div
        ref={setNodeRef}
        className="flex-1 overflow-y-auto px-3 pb-3 space-y-2"
      >
        <SortableContext
          items={posts.map((p) => p.id)}
          strategy={verticalListSortingStrategy}
        >
          {posts.map((post) => (
            <WorkflowCard
              key={post.id}
              id={post.id}
              title={post.title}
              postType={post.postType}
              isUrgent={post.isUrgent}
              createdAt={post.createdAt}
            />
          ))}
        </SortableContext>

        {posts.length === 0 && (
          <div className="flex items-center justify-center h-24 text-xs text-text-faint">
            No posts
          </div>
        )}
      </div>
    </div>
  );
}
