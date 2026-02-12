"use client";

import { useParams } from "next/navigation";
import Link from "next/link";
import { useRestate } from "@/lib/restate/client";
import type { OrganizationDetailResult } from "@/lib/restate/types";
import { BackLink } from "@/components/ui/BackLink";
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
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-10 pb-20">
        <div className="h-4 w-32 bg-border rounded mb-6 animate-pulse" />
        <div className="h-8 w-1/3 bg-border rounded mb-2 animate-pulse" />
        <div className="h-4 w-2/3 bg-border rounded mb-8 animate-pulse" />
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
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-10 pb-20">
        <BackLink href="/organizations">Back to Organizations</BackLink>
        <p className="text-text-muted">Organization not found.</p>
      </section>
    );
  }

  return (
    <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-10 pb-20">
      <BackLink href="/organizations">Back to Organizations</BackLink>

      <h1 className="text-3xl font-bold text-text-primary leading-tight tracking-tight mb-2">{data.name}</h1>
      {data.description && (
        <p className="text-text-secondary text-base leading-relaxed mb-8">
          {data.description}
        </p>
      )}

      <h2 className="text-xl font-semibold text-text-primary mb-4">
        {data.posts.length} {data.posts.length === 1 ? "Post" : "Posts"}
      </h2>

      {data.posts.length === 0 ? (
        <p className="text-text-muted">No posts yet.</p>
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
