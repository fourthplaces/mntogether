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
    <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
      <Link
        href="/"
        className="inline-flex items-center text-sm text-[#7D7D7D] hover:text-[#3D3D3D] mb-6"
      >
        &larr; Back to Home
      </Link>

      <h1 className="text-3xl font-bold text-[#3D3D3D] mb-2">Organizations</h1>
      {!isLoading && (
        <p className="text-sm text-[#7D7D7D] mb-8">
          {organizations.length} {organizations.length === 1 ? "organization" : "organizations"}
        </p>
      )}

      {isLoading ? (
        <div className="space-y-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="bg-white p-6 rounded-lg border border-[#E8DED2] animate-pulse">
              <div className="h-6 w-1/3 bg-gray-200 rounded mb-2" />
              <div className="h-4 w-full bg-gray-200 rounded mb-1" />
              <div className="h-4 w-2/3 bg-gray-200 rounded" />
            </div>
          ))}
        </div>
      ) : (
        <div className="space-y-4">
          {organizations.map((org) => (
            <Link
              key={org.id}
              href={`/organizations/${org.id}`}
              className="bg-white p-6 rounded-lg border border-[#E8DED2] hover:shadow-md transition-shadow block"
            >
              <h2 className="text-xl font-bold text-[#3D3D3D] mb-1">{org.name}</h2>
              {org.description && (
                <p className="text-[#5D5D5D] text-[0.95rem] leading-relaxed">
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
