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

  const tabClassName = (isActive: boolean) =>
    `tab ${isActive ? "tab--active" : "tab--inactive"}`;

  return (
    <div>
      {/* Title + Zip + Tabs */}
      <div className="post-feed-toolbar">
        <div className="post-feed-title-group">
          <h2 className="section-title">{title}</h2>
          <div className="zip-input-wrapper">
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
              className={`input-zip ${activeZip ? "input-zip--active" : ""}`}
            />
            {activeZip && (
              <button
                onClick={clearZip}
                className="zip-clear"
                aria-label="Clear zip code"
              >
                &times;
              </button>
            )}
          </div>
        </div>
        <div className="post-feed-tabs scrollbar-hide">
          {postTypes.map((pt) =>
            onFilterChange ? (
              <button
                key={pt.value}
                onClick={() => onFilterChange(effectivePostType === pt.value ? null : pt.value)}
                className={tabClassName(effectivePostType === pt.value)}
              >
                {pt.displayName}
              </button>
            ) : (
              <Link
                key={pt.value}
                href={`/posts?post_type=${pt.value}`}
                className={tabClassName(effectivePostType === pt.value)}
              >
                {pt.displayName}
              </Link>
            )
          )}
          {onFilterChange ? (
            <button
              onClick={() => onFilterChange(null)}
              className={tabClassName(effectivePostType == null)}
            >
              All
            </button>
          ) : (
            <Link href="/posts?post_type=all" className={tabClassName(effectivePostType == null)}>
              All
            </Link>
          )}
        </div>
      </div>

      {/* Post List */}
      <div className="post-feed-list">
        {isLoading ? (
          Array.from({ length: skeletonCount }).map((_, i) => (
            <PostCardSkeleton key={i} />
          ))
        ) : posts.length === 0 ? (
          <div className="post-feed-empty">
            <p className="text-muted-sm">
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
              <div className="post-feed-see-more">
                <Link
                  href="/posts"
                  className="btn-outline"
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
        <p className="post-feed-count">
          Showing {posts.length} of {listData?.publicPosts?.totalCount ?? posts.length} results
        </p>
      )}
    </div>
  );
}
