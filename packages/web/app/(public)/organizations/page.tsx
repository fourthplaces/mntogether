"use client";

import Link from "next/link";
import { useRestate } from "@/lib/restate/client";
import { Card } from "@/components/ui/Card";
import type { OrganizationListResult } from "@/lib/restate/types";

export default function OrganizationsPage() {
  const { data, isLoading } = useRestate<OrganizationListResult>(
    "Organizations",
    "public_list",
    {}
  );

  const organizations = data?.organizations ?? [];

  return (
    <section className="max-w-[1200px] mx-auto px-6 md:px-12 pt-8 pb-16">
      <Link
        href="/"
        className="inline-flex items-center text-sm text-text-muted hover:text-text-primary mb-6"
      >
        &larr; Back to Home
      </Link>

      <h1 className="text-3xl font-bold text-text-primary mb-2">Organizations</h1>
      {!isLoading && (
        <p className="text-sm text-text-muted mb-8">
          {organizations.length} {organizations.length === 1 ? "organization" : "organizations"}
        </p>
      )}

      {isLoading ? (
        <div className="space-y-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <Card key={i}>
              <div className="animate-pulse">
                <div className="h-6 w-1/3 bg-gray-200 rounded mb-2" />
                <div className="h-4 w-full bg-gray-200 rounded mb-1" />
                <div className="h-4 w-2/3 bg-gray-200 rounded" />
              </div>
            </Card>
          ))}
        </div>
      ) : (
        <div className="space-y-4">
          {organizations.map((org) => (
            <Link
              key={org.id}
              href={`/organizations/${org.id}`}
              className="block"
            >
              <Card variant="interactive">
                <h2 className="text-xl font-bold text-text-primary mb-1">{org.name}</h2>
                {org.description && (
                  <p className="text-text-secondary text-[0.95rem] leading-relaxed">
                    {org.description}
                  </p>
                )}
              </Card>
            </Link>
          ))}
        </div>
      )}
    </section>
  );
}
