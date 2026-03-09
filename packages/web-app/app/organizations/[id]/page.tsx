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
      <section className="page-section">
        <div className="skeleton-line skeleton" style={{ height: "1rem", width: "8rem", marginBottom: "1.5rem" }} />
        <div className="skeleton-line skeleton" style={{ height: "2rem", width: "33%", marginBottom: "0.5rem" }} />
        <div className="skeleton-line skeleton" style={{ height: "1rem", width: "66%", marginBottom: "2rem" }} />
        <div className="stack">
          {Array.from({ length: 4 }).map((_, i) => (
            <PostCardSkeleton key={i} />
          ))}
        </div>
      </section>
    );
  }

  if (!org) {
    return (
      <section className="page-section">
        <Link href="/organizations" className="back-link">
          &larr; Back to Organizations
        </Link>
        <p className="text-muted-sm">Organization not found.</p>
      </section>
    );
  }

  return (
    <section className="page-section">
      <Link href="/organizations" className="back-link">
        &larr; Back to Organizations
      </Link>

      <h1 className="page-title" style={{ marginBottom: "0.5rem" }}>{org.name}</h1>
      {org.description && (
        <p className="text-secondary" style={{ marginBottom: "2rem" }}>
          {org.description}
        </p>
      )}

      <h2 className="card-title--semi" style={{ marginBottom: "1rem" }}>
        {org.posts.length} {org.posts.length === 1 ? "Post" : "Posts"}
      </h2>

      {org.posts.length === 0 ? (
        <p className="text-muted-sm">No posts yet.</p>
      ) : (
        <div className="stack">
          {org.posts.map((post) => (
            <PostCard key={post.id} post={post} />
          ))}
        </div>
      )}
    </section>
  );
}
