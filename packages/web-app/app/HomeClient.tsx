"use client";

import { useState } from "react";
import Link from "next/link";
import { useQuery } from "urql";
import { PublicFiltersQuery } from "@/lib/graphql/public";
import { SubmitSheet } from "@/components/SubmitSheet";
import { PostFeed } from "@/components/PostFeed";

type ActiveSheet = "search" | "submit" | null;

export function HomeClient() {
  const [activeSheet, setActiveSheet] = useState<ActiveSheet>(null);

  const [{ data: filtersData }] = useQuery({ query: PublicFiltersQuery });

  const postTypes = filtersData?.publicFilters?.postTypes ?? [];

  return (
    <div>
      {/* Post type index */}
      {postTypes.length > 0 && (
        <section className="max-w-[960px] mx-auto px-4 pt-8 pb-6">
          <h2 className="text-lg font-bold text-text-primary mb-3">Browse by type</h2>
          <ul className="space-y-1">
            {postTypes.map((pt) => (
              <li key={pt.value}>
                <Link
                  href={`/posts?post_type=${pt.value}`}
                  className="text-link hover:underline"
                >
                  {pt.displayName}
                </Link>
                {pt.description && (
                  <span className="text-text-muted text-sm"> &mdash; {pt.description}</span>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Recent posts */}
      <section className="max-w-[960px] mx-auto px-4 pb-10">
        <PostFeed title="Recent posts" showSeeMore />
      </section>

      {/* Bottom Sheets */}
      <SubmitSheet isOpen={activeSheet === "submit"} onClose={() => setActiveSheet(null)} />
    </div>
  );
}
