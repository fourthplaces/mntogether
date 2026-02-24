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
    `px-3 py-1 text-sm border whitespace-nowrap ${
      isActive
        ? "bg-action text-text-on-action border-action"
        : "bg-transparent text-text-secondary border-border hover:border-border-strong"
    }`;

  return (
    <div>
      {/* Title + Zip + Tabs */}
      <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-8 gap-4">
        <div className="flex items-center gap-3">
          <h2 className="text-2xl font-bold text-text-primary">{title}</h2>
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
              className={`w-28 px-4 py-2 border text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-action ${
                activeZip ? "border-action bg-surface-muted" : "border-border"
              }`}
            />
            {activeZip && (
              <button
                onClick={clearZip}
                className="absolute right-3 text-text-muted hover:text-text-primary text-lg leading-none"
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
            <p className="text-text-muted text-sm">
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
                  className="inline-block px-4 py-2 text-sm border border-border text-text-secondary hover:border-border-strong"
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
        <p className="text-center text-sm text-text-muted mt-6">
          Showing {posts.length} of {listData?.publicPosts?.totalCount ?? posts.length} results
        </p>
      )}
    </div>
  );
}
