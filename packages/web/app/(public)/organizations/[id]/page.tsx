"use client";

import { useParams } from "next/navigation";
import Link from "next/link";
import { useRestate } from "@/lib/restate/client";
import type { OrganizationDetailResult } from "@/lib/restate/types";
import { PostCard, PostCardSkeleton } from "@/components/public/PostCard";

export default function OrganizationDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { data, isLoading } = useRestate<OrganizationDetailResult>(
    "Organizations",
    "public_get",
    { id }
  );

  if (isLoading) {
    return (
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
        <div className="h-4 w-32 bg-gray-200 rounded mb-6 animate-pulse" />
        <div className="h-8 w-1/3 bg-gray-200 rounded mb-2 animate-pulse" />
        <div className="h-4 w-2/3 bg-gray-200 rounded mb-8 animate-pulse" />
        <div className="space-y-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <PostCardSkeleton key={i} />
          ))}
        </div>
      </section>
    );
  }

  if (!data) {
    return (
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
        <Link
          href="/organizations"
          className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-6"
        >
          &larr; Back to Organizations
        </Link>
        <p className="text-[#7D7D7D]">Organization not found.</p>
      </section>
    );
  }

  return (
    <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
      <Link
        href="/organizations"
        className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-6"
      >
        &larr; Back to Organizations
      </Link>

      <h1 className="text-3xl font-bold text-[#3D3D3D] mb-2">{data.name}</h1>
      {data.description && (
        <p className="text-[#5D5D5D] text-[0.95rem] leading-relaxed mb-8">
          {data.description}
        </p>
      )}

      <h2 className="text-xl font-semibold text-[#3D3D3D] mb-4">
        {data.posts.length} {data.posts.length === 1 ? "Post" : "Posts"}
      </h2>

      {data.posts.length === 0 ? (
        <p className="text-[#7D7D7D]">No posts yet.</p>
      ) : (
        <div className="space-y-4">
          {data.posts.map((post) => (
            <PostCard key={post.id} post={post} />
          ))}
        </div>
      )}
    </section>
  );
}
