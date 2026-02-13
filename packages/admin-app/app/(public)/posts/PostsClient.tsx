"use client";

import Link from "next/link";
import { useSearchParams, useRouter } from "next/navigation";
import { PostFeed } from "@/components/public/PostFeed";

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
    <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
      {/* Back link */}
      <Link
        href="/"
        className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-6"
      >
        &larr; Back to Home
      </Link>

      <PostFeed
        title="Community Resources"
        activePostType={postType}
        onFilterChange={setFilter}
        showResultCount
        skeletonCount={8}
      />
    </section>
  );
}
