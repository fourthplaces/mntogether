"use client";

import { useParams } from "next/navigation";
import Link from "next/link";
import { useQuery } from "urql";
import { PublicOrganizationQuery } from "@/lib/graphql/public";
import { PostCard, PostCardSkeleton } from "@/components/PostCard";

export default function OrganizationDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [{ data, fetching: isLoading }] = useQuery({
    query: PublicOrganizationQuery,
    variables: { id },
  });

  const org = data?.publicOrganization;

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

  if (!org) {
    return (
      <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-10 pb-20">
        <Link href="/organizations" className="inline-block text-sm text-text-secondary hover:text-text-primary mb-6">
          &larr; Back to Organizations
        </Link>
        <p className="text-text-muted">Organization not found.</p>
      </section>
    );
  }

  return (
    <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-10 pb-20">
      <Link href="/organizations" className="inline-block text-sm text-text-secondary hover:text-text-primary mb-6">
        &larr; Back to Organizations
      </Link>

      <h1 className="text-3xl font-bold text-text-primary leading-tight tracking-tight mb-2">{org.name}</h1>
      {org.description && (
        <p className="text-text-secondary text-base leading-relaxed mb-8">
          {org.description}
        </p>
      )}

      <h2 className="text-xl font-semibold text-text-primary mb-4">
        {org.posts.length} {org.posts.length === 1 ? "Post" : "Posts"}
      </h2>

      {org.posts.length === 0 ? (
        <p className="text-text-muted">No posts yet.</p>
      ) : (
        <div className="space-y-4">
          {org.posts.map((post) => (
            <PostCard key={post.id} post={post} />
          ))}
        </div>
      )}
    </section>
  );
}
