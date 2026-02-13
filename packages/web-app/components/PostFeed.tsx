"use client";

import { useState } from "react";
import Link from "next/link";
import { useQuery } from "urql";
import { PublicPostsQuery, PublicFiltersQuery } from "@/lib/graphql/public";
import { PostCard, PostCardSkeleton } from "@/components/PostCard";

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
  const [zipInput, setZipInput] = useState("");
  const [activeZip, setActiveZip] = useState<string | null>(null);

  const [{ data: filtersData }] = useQuery({ query: PublicFiltersQuery });

  const postTypes = filtersData?.publicFilters?.postTypes ?? [];

  // "all" means user explicitly chose All; null/undefined means default to first tab
  const effectivePostType =
    activePostType === "all" ? null : (activePostType ?? postTypes[0]?.value ?? null);

  const [{ data: listData, fetching: isLoading }] = useQuery({
    query: PublicPostsQuery,
    variables: {
      postType: effectivePostType,
      zipCode: activeZip,
      radiusMiles: activeZip ? 25 : undefined,
    },
  });

  const posts = listData?.publicPosts?.posts ?? [];

  const handleZipSearch = () => {
    const trimmed = zipInput.trim();
    if (/^\d{5}$/.test(trimmed)) {
      setActiveZip(trimmed);
    }
  };

  const clearZip = () => {
    setZipInput("");
    setActiveZip(null);
  };

  const tabClass = (isActive: boolean) =>
    `px-5 py-2 rounded-full text-sm font-semibold border transition-all whitespace-nowrap ${
      isActive
        ? "bg-[#3D3D3D] text-white border-[#3D3D3D]"
        : "bg-transparent text-[#5D5D5D] border-[#C4B8A0] hover:border-[#3D3D3D]"
    }`;

  return (
    <div>
      {/* Title + Zip + Tabs */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-8 gap-4">
        <div className="flex items-center gap-3">
          <h2 className="text-3xl font-bold text-[#3D3D3D]">{title}</h2>
          <div className="relative flex items-center">
            <input
              type="text"
              inputMode="numeric"
              placeholder="Zip code"
              value={zipInput}
              onChange={(e) => {
                const val = e.target.value.replace(/\D/g, "").slice(0, 5);
                setZipInput(val);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleZipSearch();
              }}
              className={`w-28 px-4 py-2 rounded-full border text-sm text-[#3D3D3D] placeholder-[#9D9D9D] focus:outline-none focus:border-[#3D3D3D] transition-colors ${
                activeZip ? "border-[#3D3D3D] bg-[#F5F1E8]" : "border-[#C4B8A0]"
              }`}
            />
            {activeZip && (
              <button
                onClick={clearZip}
                className="absolute right-3 text-[#9D9D9D] hover:text-[#3D3D3D] text-lg leading-none"
                aria-label="Clear zip code"
              >
                &times;
              </button>
            )}
          </div>
        </div>
        <div className="flex gap-3 overflow-x-auto scrollbar-hide">
          {postTypes.map((pt) =>
            onFilterChange ? (
              <button
                key={pt.value}
                onClick={() => onFilterChange(effectivePostType === pt.value ? null : pt.value)}
                className={tabClass(effectivePostType === pt.value)}
              >
                {pt.displayName}
              </button>
            ) : (
              <Link
                key={pt.value}
                href={`/posts?post_type=${pt.value}`}
                className={tabClass(effectivePostType === pt.value)}
              >
                {pt.displayName}
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
            <p className="text-[#7D7D7D] text-sm">
              {activeZip
                ? `No resources found within 25 miles of ${activeZip}. Try a different zip code.`
                : "No resources found. Try a different filter."}
            </p>
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
          Showing {posts.length} of {listData?.publicPosts?.totalCount ?? posts.length} results
        </p>
      )}
    </div>
  );
}
