"use client";

import Link from "next/link";
import { useQuery } from "urql";
import { PublicOrganizationsQuery } from "@/lib/graphql/public";

export default function OrganizationsPage() {
  const [{ data, fetching: isLoading }] = useQuery({
    query: PublicOrganizationsQuery,
  });

  const organizations = data?.publicOrganizations ?? [];

  return (
    <section className="page-section">
      <Link href="/" className="back-link">
        &larr; Back to Home
      </Link>

      <h1 className="page-title" style={{ marginBottom: "0.5rem" }}>Organizations</h1>
      {!isLoading && (
        <p className="text-muted-sm" style={{ marginBottom: "2rem" }}>
          {organizations.length} {organizations.length === 1 ? "organization" : "organizations"}
        </p>
      )}

      {isLoading ? (
        <div className="stack">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="card">
              <div className="skeleton">
                <div className="skeleton-line" style={{ height: "1.5rem", width: "33%", marginBottom: "0.5rem" }} />
                <div className="skeleton-line" style={{ height: "1rem", width: "100%", marginBottom: "0.25rem" }} />
                <div className="skeleton-line" style={{ height: "1rem", width: "66%" }} />
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="stack">
          {organizations.map((org) => (
            <Link
              key={org.id}
              href={`/organizations/${org.id}`}
              className="card--interactive"
            >
              <h2 className="card-title" style={{ marginBottom: "0.25rem" }}>{org.name}</h2>
              {org.description && (
                <p className="text-secondary">
                  {org.description}
                </p>
              )}
            </Link>
          ))}
        </div>
      )}
    </section>
  );
}
