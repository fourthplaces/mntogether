"use client";

import { useState, useEffect } from "react";
import { PostCard, PostCardSkeleton } from "@/components/public/PostCard";
import { SuggestedPrompts } from "@/components/public/SuggestedPrompts";
import type { ToolResult } from "@/lib/hooks/usePublicChatStream";
import type { Post } from "@/lib/types";

type ContentState = "welcome" | "searching" | "results" | "empty" | "detail";

interface ContentPanelProps {
  toolResults: ToolResult[];
  isSearching: boolean;
  onSuggestedPrompt: (query: string) => void;
}

/** Map raw tool result data to a Post-like shape for PostCard rendering. */
function toolResultsToPosts(toolResults: ToolResult[]): Post[] {
  const posts: Post[] = [];
  for (const tr of toolResults) {
    if (!Array.isArray(tr.results)) continue;
    for (const r of tr.results as Record<string, unknown>[]) {
      posts.push({
        id: (r.post_id as string) || "",
        title: (r.title as string) || "",
        description: (r.description as string) || "",
        tldr: r.tldr as string | undefined,
        category: r.category as string | undefined,
        postType: r.post_type as Post["postType"],
        location: r.location as string | undefined,
        sourceUrl: r.source_url as string | undefined,
        status: "ACTIVE",
        createdAt: new Date().toISOString(),
      });
    }
  }
  return posts;
}

export function ContentPanel({
  toolResults,
  isSearching,
  onSuggestedPrompt,
}: ContentPanelProps) {
  const [contentState, setContentState] = useState<ContentState>("welcome");
  const [selectedPost, setSelectedPost] = useState<Post | null>(null);
  const [posts, setPosts] = useState<Post[]>([]);

  // Derive state from props
  useEffect(() => {
    if (isSearching) {
      setContentState("searching");
      setSelectedPost(null);
      return;
    }

    if (toolResults.length > 0) {
      const mapped = toolResultsToPosts(toolResults);
      setPosts(mapped);
      setContentState(mapped.length > 0 ? "results" : "empty");
      setSelectedPost(null);
    }
  }, [toolResults, isSearching]);

  const handleStartOver = () => {
    setContentState("welcome");
    setPosts([]);
    setSelectedPost(null);
  };

  // ---- Welcome ----
  if (contentState === "welcome") {
    return (
      <div className="flex flex-col items-center justify-center h-full p-6">
        <div className="max-w-2xl w-full text-center mb-8">
          <h1 className="text-3xl sm:text-4xl font-bold text-gray-900 mb-3">
            MN Together
          </h1>
          <p className="text-gray-600 text-lg mb-8">
            Find services, volunteer opportunities, and community resources
            across Minnesota.
          </p>
          <SuggestedPrompts onSelect={onSuggestedPrompt} />
        </div>
      </div>
    );
  }

  // ---- Searching ----
  if (contentState === "searching") {
    return (
      <div className="p-6">
        <div className="flex items-center gap-2 mb-6">
          <div className="h-2 w-2 bg-blue-500 rounded-full animate-pulse" />
          <p className="text-sm text-gray-500">Searching resources...</p>
        </div>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {[...Array(6)].map((_, i) => (
            <PostCardSkeleton key={i} />
          ))}
        </div>
      </div>
    );
  }

  // ---- Detail ----
  if (contentState === "detail" && selectedPost) {
    return (
      <div className="p-6 max-w-2xl mx-auto">
        <button
          onClick={() => setContentState("results")}
          className="inline-flex items-center gap-1 text-sm text-blue-600 hover:text-blue-700 mb-4"
        >
          <svg
            className="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          Back to results
        </button>
        <div className="bg-white rounded-xl border border-gray-200 p-6">
          <h2 className="text-2xl font-bold text-gray-900 mb-2">
            {selectedPost.title}
          </h2>
          {selectedPost.location && (
            <p className="text-sm text-gray-500 mb-4 flex items-center gap-1">
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
                />
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"
                />
              </svg>
              {selectedPost.location}
            </p>
          )}
          <div className="prose prose-sm max-w-none text-gray-700 mb-6">
            <p>{selectedPost.description}</p>
          </div>
          {selectedPost.sourceUrl && (
            <a
              href={selectedPost.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              <svg
                className="w-4 h-4"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                />
              </svg>
              Visit Website
            </a>
          )}
        </div>
      </div>
    );
  }

  // ---- Empty ----
  if (contentState === "empty") {
    return (
      <div className="flex flex-col items-center justify-center h-full p-6 text-center">
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-gray-100 mb-4">
          <svg
            className="w-8 h-8 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
        </div>
        <h3 className="text-lg font-medium text-gray-900 mb-2">
          No matches found
        </h3>
        <p className="text-gray-500 mb-4 max-w-md">
          Try a different search, or call{" "}
          <a href="tel:211" className="text-blue-600 font-medium">
            211
          </a>{" "}
          for help finding resources in Minnesota.
        </p>
        <button
          onClick={handleStartOver}
          className="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors text-sm"
        >
          Start Over
        </button>
      </div>
    );
  }

  // ---- Results ----
  return (
    <div className="p-6 overflow-y-auto">
      <div className="flex items-center justify-between mb-4">
        <p className="text-sm text-gray-500">
          Found{" "}
          <span className="font-medium text-gray-900">{posts.length}</span>{" "}
          resource{posts.length !== 1 ? "s" : ""}
        </p>
        <button
          onClick={handleStartOver}
          className="text-sm text-blue-600 hover:text-blue-700"
        >
          Start over
        </button>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {posts.map((post, i) => (
          <div
            key={post.id || i}
            className="cursor-pointer"
            onClick={() => {
              setSelectedPost(post);
              setContentState("detail");
            }}
          >
            <PostCard post={post} />
          </div>
        ))}
      </div>
    </div>
  );
}
