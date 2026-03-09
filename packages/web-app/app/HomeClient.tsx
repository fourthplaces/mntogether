"use client";

import Link from "next/link";
import { useQuery } from "urql";
import { PublicFiltersQuery } from "@/lib/graphql/public";
import { PostFeed } from "@/components/PostFeed";

export function HomeClient() {
  const [{ data: filtersData }] = useQuery({ query: PublicFiltersQuery });

  const postTypes = filtersData?.publicFilters?.postTypes ?? [];

  return (
    <div>
      {/* Post type index */}
      {postTypes.length > 0 && (
        <section className="page-section--home">
          <h2 className="browse-title">Browse by type</h2>
          <ul className="browse-list">
            {postTypes.map((pt) => (
              <li key={pt.value}>
                <Link
                  href={`/posts?post_type=${pt.value}`}
                  className="browse-link"
                >
                  {pt.displayName}
                </Link>
                {pt.description && (
                  <span className="browse-description"> &mdash; {pt.description}</span>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Recent posts */}
      <section className="page-section--home-feed">
        <PostFeed title="Recent posts" showSeeMore />
      </section>

    </div>
  );
}
