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
        <p className="text-secondary" style={{ marginBottom: "1.25rem" }}>
          {org.description}
        </p>
      )}

      {/* Public platform links. The server already filters out is_public=false
       * links; whatever comes down here is safe to render. */}
      {org.links && org.links.length > 0 && (
        <div
          style={{
            display: "flex",
            flexWrap: "wrap",
            gap: "0.5rem",
            marginBottom: "2rem",
          }}
        >
          {org.links.map((link) => (
            <a
              key={link.id}
              href={link.url}
              target="_blank"
              rel="noopener noreferrer"
              className="btn-outline"
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: "0.375rem",
                padding: "0.375rem 0.75rem",
              }}
            >
              {link.platformEmoji && <span aria-hidden>{link.platformEmoji}</span>}
              <span>{link.platformLabel ?? link.platform}</span>
            </a>
          ))}
        </div>
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
