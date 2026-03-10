"use client";

import Link from "next/link";
import { useSearchParams, useRouter } from "next/navigation";
import { PostFeed } from "@/components/PostFeed";

export function PostsClient() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const postType = searchParams.get("post_type");

  const setFilter = (value: string | null) => {
    const params = new URLSearchParams();
    if (value) {
      params.set("post_type", value);
    } else {
      params.set("post_type", "all");
    }
    router.replace(`/posts?${params}`);
  };

  return (
    <section className="page-section">
      <Link href="/" className="back-link">
        &larr; Back to Home
      </Link>

      <PostFeed
        title="Posts"
        activePostType={postType}
        onFilterChange={setFilter}
        showResultCount
        skeletonCount={8}
      />
    </section>
  );
}
