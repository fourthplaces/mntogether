"use client";

import { useState } from "react";
import Link from "next/link";
import type { PublicPostResult, PostTypeOption } from "@/lib/restate/types";

function formatCategory(value: string): string {
  return value
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

export function PostCard({ post }: { post: PublicPostResult; postTypes?: PostTypeOption[] }) {
  const [showUrgent, setShowUrgent] = useState(false);
  const postTypeTag = post.tags.find((t) => t.kind === "post_type");
  const displayTags = post.tags.filter((t) => t.kind !== "post_type");
  const urgentNotes = post.urgent_notes ?? [];

  return (
    <>
      <Link
        href={`/posts/${post.id}`}
        className="bg-white p-6 rounded-lg border border-[#E8DED2] hover:shadow-md transition-shadow block"
      >
        {post.organization_name && (
          <p className="text-xs font-medium text-[#7D7D7D] uppercase tracking-wide mb-0.5">
            {post.organization_name}
          </p>
        )}
        <div className="flex items-center gap-2 mb-1">
          <h3 className="text-xl font-bold text-[#3D3D3D]">{post.title}</h3>
          {urgentNotes.length > 0 && (
            <button
              type="button"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setShowUrgent(true);
              }}
              className="px-2.5 py-0.5 text-xs font-medium rounded-full bg-red-100 text-red-800 shrink-0 hover:bg-red-200 transition-colors"
            >
              Urgent
            </button>
          )}
        </div>
        {(post.location || post.distance_miles != null) && (
          <p className="text-sm text-[#7D7D7D] mb-1">
            {post.location}
            {post.distance_miles != null && (
              <span className="ml-2 text-[#5D8A68] font-medium">
                {post.distance_miles < 1
                  ? "< 1 mi"
                  : `${Math.round(post.distance_miles)} mi`}
              </span>
            )}
          </p>
        )}
        <p className="text-[#5D5D5D] text-[0.95rem] leading-relaxed mb-3">
          {post.summary || post.description}
        </p>
        <div className="flex flex-wrap gap-2">
          {postTypeTag && (
            <span
              title={`${postTypeTag.kind}: ${postTypeTag.value}`}
              className={`px-3 py-1 rounded-full text-xs font-medium ${!postTypeTag.color ? "bg-[#F5F1E8] text-[#5D5D5D]" : ""}`}
              style={postTypeTag.color ? { backgroundColor: postTypeTag.color + "20", color: postTypeTag.color } : undefined}
            >
              {postTypeTag.display_name || formatCategory(postTypeTag.value)}
            </span>
          )}
          {displayTags.map((tag) => (
            <span
              key={tag.value}
              title={`${tag.kind}: ${tag.value}`}
              className={`px-3 py-1 rounded-full text-xs font-medium ${!tag.color ? "bg-[#F5F1E8] text-[#5D5D5D]" : ""}`}
              style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
            >
              {tag.display_name || formatCategory(tag.value)}
            </span>
          ))}
        </div>
      </Link>

      {showUrgent && (
        <>
          <div
            className="fixed inset-0 bg-black/40 z-40"
            onClick={() => setShowUrgent(false)}
          />
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div className="bg-white rounded-xl shadow-xl w-full max-w-md">
              <div className="flex items-center justify-between px-5 py-4 border-b border-stone-200">
                <div className="flex items-center gap-2">
                  <span className="px-2.5 py-0.5 text-xs font-medium rounded-full bg-red-100 text-red-800">
                    Urgent
                  </span>
                  <h2 className="text-lg font-semibold text-stone-900">Notes</h2>
                </div>
                <button
                  onClick={() => setShowUrgent(false)}
                  className="text-stone-400 hover:text-stone-600 text-xl leading-none"
                >
                  &times;
                </button>
              </div>
              <div className="p-5 space-y-3 max-h-80 overflow-y-auto">
                {urgentNotes.map((note, i) => (
                  <div key={i}>
                    {note.cta_text && (
                      <p className="text-sm font-semibold text-red-900">{note.cta_text}</p>
                    )}
                    <p className="text-sm text-stone-700 leading-relaxed">{note.content}</p>
                  </div>
                ))}
              </div>
              <div className="px-5 py-3 border-t border-stone-200 flex justify-end">
                <button
                  onClick={() => setShowUrgent(false)}
                  className="px-4 py-2 text-sm font-medium text-stone-600 hover:text-stone-800"
                >
                  Close
                </button>
              </div>
            </div>
          </div>
        </>
      )}
    </>
  );
}

export function PostCardSkeleton() {
  return (
    <div className="bg-white p-6 rounded-lg border border-[#E8DED2] animate-pulse">
      <div className="h-6 w-3/4 bg-gray-200 rounded mb-2" />
      <div className="h-4 w-1/3 bg-gray-200 rounded mb-2" />
      <div className="h-4 w-full bg-gray-200 rounded mb-1" />
      <div className="h-4 w-5/6 bg-gray-200 rounded mb-3" />
      <div className="h-6 w-20 bg-gray-200 rounded-full" />
    </div>
  );
}
