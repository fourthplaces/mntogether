"use client";

import { useState, useMemo } from "react";
import { useRestate } from "@/lib/restate/client";
import { PostListItem, PostListItemSkeleton } from "@/components/public/PostCard";
import type {
  PublicListResult,
  PublicFiltersResult,
} from "@/lib/restate/types";

type AudienceFilter = "need_help" | "want_to_give" | "events" | null;

const AUDIENCE_BUTTONS: {
  key: AudienceFilter & string;
  label: string;
  icon: string;
}[] = [
  { key: "need_help", label: "I Need Help", icon: "\u{1F932}" },
  { key: "want_to_give", label: "I Want to Give", icon: "\u{1F49B}" },
  { key: "events", label: "Community Events", icon: "\u{1F4C5}" },
];

export function HomeClient() {
  const [audience, setAudience] = useState<AudienceFilter>(null);
  const [category, setCategory] = useState<string | null>(null);

  // Build request body — only include non-null filters
  const requestBody = useMemo(() => {
    const body: Record<string, unknown> = {};
    if (audience) body.audience = audience;
    if (category) body.category = category;
    return body;
  }, [audience, category]);

  const { data: listData, isLoading: listLoading } =
    useRestate<PublicListResult>("Posts", "public_list", requestBody);

  const { data: filtersData } =
    useRestate<PublicFiltersResult>("Posts", "public_filters", {});

  const posts = listData?.posts ?? [];
  const totalCount = listData?.total_count ?? 0;
  const categories = filtersData?.categories ?? [];

  const toggleAudience = (key: AudienceFilter & string) => {
    if (audience === key) {
      setAudience(null);
      setCategory(null);
    } else {
      setAudience(key);
      setCategory(null);
    }
  };

  const toggleCategory = (value: string) => {
    setCategory(category === value ? null : value);
  };

  const clearAll = () => {
    setAudience(null);
    setCategory(null);
  };

  const hasFilters = audience !== null || category !== null;

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

      {/* Action Buttons */}
      <div className="max-w-2xl mx-auto px-4 -mt-2">
        <div className="flex flex-col sm:flex-row gap-3">
          {AUDIENCE_BUTTONS.map((btn) => {
            const isActive = audience === btn.key;
            return (
              <button
                key={btn.key}
                onClick={() => toggleAudience(btn.key)}
                className={`flex-1 py-3 px-4 rounded-xl text-sm font-semibold transition-all border-2 ${
                  isActive
                    ? "bg-blue-600 text-white border-blue-600 shadow-md"
                    : "bg-white text-gray-700 border-gray-200 hover:border-blue-300 hover:bg-blue-50"
                }`}
              >
                <span className="mr-1.5">{btn.icon}</span>
                {btn.label}
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
      <div className="max-w-2xl mx-auto border-t border-gray-200">
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
            {hasFilters && (
              <button
                onClick={clearAll}
                className="mt-3 text-blue-600 text-sm hover:underline"
              >
                Clear all filters
              </button>
            )}
          </div>
        ) : (
          posts.map((post) => <PostListItem key={post.id} post={post} />)
        )}
      </div>
    </div>
  );
}
