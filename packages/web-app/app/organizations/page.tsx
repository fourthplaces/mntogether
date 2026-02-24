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
    <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-10 pb-20">
      <Link href="/" className="inline-block text-sm text-text-secondary hover:text-text-primary mb-6">
        &larr; Back to Home
      </Link>

      <h1 className="text-3xl font-bold text-text-primary leading-tight tracking-tight mb-2">Organizations</h1>
      {!isLoading && (
        <p className="text-sm text-text-muted mb-8">
          {organizations.length} {organizations.length === 1 ? "organization" : "organizations"}
        </p>
      )}

      {isLoading ? (
        <div className="space-y-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="bg-surface-raised border border-border p-6">
              <div className="animate-pulse">
                <div className="h-6 w-1/3 bg-border rounded mb-2" />
                <div className="h-4 w-full bg-border rounded mb-1" />
                <div className="h-4 w-2/3 bg-border rounded" />
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="space-y-4">
          {organizations.map((org) => (
            <Link
              key={org.id}
              href={`/organizations/${org.id}`}
              className="block bg-surface-raised border border-border p-6 hover:border-border-strong"
            >
              <h2 className="text-xl font-bold text-text-primary mb-1">{org.name}</h2>
              {org.description && (
                <p className="text-text-secondary text-base leading-relaxed">
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
