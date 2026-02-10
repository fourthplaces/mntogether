"use client";

import Link from "next/link";
import { useRestate } from "@/lib/restate/client";
import { PostCard, PostCardSkeleton } from "@/components/public/PostCard";
import type {
  PublicListResult,
  PublicFiltersResult,
} from "@/lib/restate/types";

interface PostFeedProps {
  title: string;
  activePostType?: string | null;
  onFilterChange?: (value: string | null) => void;
  showSeeMore?: boolean;
  showResultCount?: boolean;
  skeletonCount?: number;
}

export function PostFeed({
  title,
  activePostType,
  onFilterChange,
  showSeeMore = false,
  showResultCount = false,
  skeletonCount = 6,
}: PostFeedProps) {
  const { data: filtersData } =
    useRestate<PublicFiltersResult>("Posts", "public_filters", {});

  const postTypes = filtersData?.post_types ?? [];

  // "all" means user explicitly chose All; null/undefined means default to first tab
  const effectivePostType =
    activePostType === "all" ? null : (activePostType ?? postTypes[0]?.value ?? null);

  const requestBody = effectivePostType ? { post_type: effectivePostType } : {};

  const { data: listData, isLoading } =
    useRestate<PublicListResult>("Posts", "public_list", requestBody);

  const posts = listData?.posts ?? [];

  const tabClass = (isActive: boolean) =>
    `px-5 py-2 rounded-full text-sm font-semibold border transition-all whitespace-nowrap ${
      isActive
        ? "bg-[#3D3D3D] text-white border-[#3D3D3D]"
        : "bg-transparent text-[#5D5D5D] border-[#C4B8A0] hover:border-[#3D3D3D]"
    }`;

  return (
    <div>
      {/* Title + Tabs */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-8 gap-4">
        <h2 className="text-3xl font-bold text-[#3D3D3D]">{title}</h2>
        <div className="flex gap-3 overflow-x-auto scrollbar-hide">
          {postTypes.map((pt) =>
            onFilterChange ? (
              <button
                key={pt.value}
                onClick={() => onFilterChange(effectivePostType === pt.value ? null : pt.value)}
                className={tabClass(effectivePostType === pt.value)}
              >
                {pt.display_name}
              </button>
            ) : (
              <Link
                key={pt.value}
                href={`/posts?post_type=${pt.value}`}
                className={tabClass(effectivePostType === pt.value)}
              >
                {pt.display_name}
              </Link>
            )
          )}
          {onFilterChange ? (
            <button
              onClick={() => onFilterChange(null)}
              className={tabClass(effectivePostType == null)}
            >
              All
            </button>
          ) : (
            <Link href="/posts?post_type=all" className={tabClass(effectivePostType == null)}>
              All
            </Link>
          )}
        </div>
      </div>

      {/* Post List */}
      <div className="flex flex-col gap-4">
        {isLoading ? (
          Array.from({ length: skeletonCount }).map((_, i) => (
            <PostCardSkeleton key={i} />
          ))
        ) : posts.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-[#7D7D7D] text-sm">No resources found. Try a different filter.</p>
          </div>
        ) : (
          <>
            {posts.map((post) => (
              <PostCard key={post.id} post={post} />
            ))}
            {showSeeMore && (
              <div className="text-center pt-4">
                <Link
                  href="/posts"
                  className="inline-block px-6 py-3 rounded-full border border-[#C4B8A0] text-[#5D5D5D] font-semibold text-sm hover:border-[#3D3D3D] hover:text-[#3D3D3D] transition-all"
                >
                  See More
                </Link>
              </div>
            )}
          </>
        )}
      </div>

      {/* Result count */}
      {showResultCount && !isLoading && posts.length > 0 && (
        <p className="text-center text-sm text-[#7D7D7D] mt-6">
          Showing {posts.length} of {listData?.total_count ?? posts.length} results
        </p>
      )}
    </div>
  );
}
