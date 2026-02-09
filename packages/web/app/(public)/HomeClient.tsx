"use client";

import { useState, useMemo } from "react";
import { useRestate } from "@/lib/restate/client";
import { PostListItem, PostListItemSkeleton } from "@/components/public/PostCard";
import { SubmitSheet } from "@/components/public/SubmitSheet";
import type {
  PublicListResult,
  PublicFiltersResult,
} from "@/lib/restate/types";

type ActiveSheet = "search" | "submit" | null;

export function HomeClient() {
  const [postType, setPostType] = useState<string | null>(null);
  const [category, setCategory] = useState<string | null>(null);
  const [activeSheet, setActiveSheet] = useState<ActiveSheet>(null);

  // Build request body — only include non-null filters
  const requestBody = useMemo(() => {
    const body: Record<string, unknown> = {};
    if (postType) body.post_type = postType;
    if (category) body.category = category;
    return body;
  }, [postType, category]);

  const { data: listData, isLoading: listLoading } =
    useRestate<PublicListResult>("Posts", "public_list", requestBody);

  const { data: filtersData } =
    useRestate<PublicFiltersResult>("Posts", "public_filters", {});

  const posts = listData?.posts ?? [];
  const totalCount = listData?.total_count ?? 0;
  const categories = filtersData?.categories ?? [];
  const postTypes = filtersData?.post_types ?? [];

  const togglePostType = (value: string) => {
    if (postType === value) {
      setPostType(null);
      setCategory(null);
    } else {
      setPostType(value);
      setCategory(null);
    }
  };

  const toggleCategory = (value: string) => {
    setCategory(category === value ? null : value);
  };

  const clearAll = () => {
    setPostType(null);
    setCategory(null);
  };

  const hasFilters = postType !== null || category !== null;

  return (
    <div className="min-h-screen bg-white">
      {/* Hero */}
      <header className="bg-gradient-to-b from-blue-50 to-white pt-12 pb-8 px-4">
        <div className="max-w-2xl mx-auto text-center">
          <h1 className="text-3xl sm:text-4xl font-bold text-gray-900 tracking-tight">
            MN Together
          </h1>
          <p className="mt-2 text-lg text-gray-500">
            Find help. Give help. Come together.
          </p>
        </div>
      </header>

      {/* Post Type Buttons */}
      <div className="max-w-2xl mx-auto px-4 -mt-2">
        <div className="flex flex-col sm:flex-row gap-3">
          {postTypes.map((pt) => {
            const isActive = postType === pt.value;
            const color = pt.color || "#3b82f6";
            return (
              <button
                key={pt.value}
                onClick={() => togglePostType(pt.value)}
                className={`flex-1 py-3 px-4 rounded-xl text-center transition-all border-2 shadow-sm ${
                  isActive ? "text-white shadow-md" : "text-gray-700 hover:shadow-md"
                }`}
                style={
                  isActive
                    ? { backgroundColor: color, borderColor: color }
                    : { backgroundColor: color + "10", borderColor: color + "40" }
                }
              >
                <span className="text-base font-semibold">
                  {pt.emoji && <span className="mr-1.5">{pt.emoji}</span>}
                  {pt.display_name}
                </span>
              </button>
            );
          })}
        </div>
      </div>

      {/* Category Pills */}
      {categories.length > 0 && (
        <div className="max-w-2xl mx-auto px-4 mt-5">
          <div className="flex flex-wrap gap-2">
            {categories.map((cat) => {
              const isActive = category === cat.value;
              return (
                <button
                  key={cat.value}
                  onClick={() => toggleCategory(cat.value)}
                  className={`px-3 py-1.5 rounded-full text-xs font-medium transition-all ${
                    isActive
                      ? "bg-blue-600 text-white"
                      : "bg-gray-100 text-gray-600 hover:bg-gray-200"
                  }`}
                >
                  {cat.display_name}
                  <span className="ml-1 opacity-60">{cat.count}</span>
                </button>
              );
            })}
            {hasFilters && (
              <button
                onClick={clearAll}
                className="px-3 py-1.5 rounded-full text-xs font-medium text-gray-400 hover:text-gray-600 transition-colors"
              >
                Clear all
              </button>
            )}
          </div>
        </div>
      )}

      {/* Results header */}
      <div className="max-w-2xl mx-auto px-4 mt-6 mb-2">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-gray-500">
            {listLoading
              ? "Loading..."
              : `Showing ${posts.length} of ${totalCount} resources`}
          </h2>
        </div>
      </div>

      {/* Post List */}
      <div className="max-w-2xl mx-auto border-t border-gray-200 pb-24">
        {listLoading ? (
          <>
            {Array.from({ length: 6 }).map((_, i) => (
              <PostListItemSkeleton key={i} />
            ))}
          </>
        ) : posts.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-gray-400 text-sm">
              No resources found. Try a different filter.
            </p>
          </div>
        ) : (
          posts.map((post) => <PostListItem key={post.id} post={post} />)
        )}
      </div>

      {/* Floating Action Buttons */}
      <div className="fixed bottom-6 left-1/2 -translate-x-1/2 flex items-center gap-3 z-40">
        <button
          onClick={() => setActiveSheet("submit")}
          className="px-5 py-3 bg-green-200 text-green-800 rounded-full shadow-lg hover:bg-green-300 transition-colors flex items-center gap-2 font-medium text-sm"
          aria-label="Share a resource"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
          Share a Community Resource
        </button>
      </div>

      {/* Bottom Sheets */}
      <SubmitSheet isOpen={activeSheet === "submit"} onClose={() => setActiveSheet(null)} />
    </div>
  );
}
