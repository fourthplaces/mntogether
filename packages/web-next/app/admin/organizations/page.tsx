"use client";

import { useGraphQL } from "@/lib/graphql/client";
import { GET_ORGANIZATIONS } from "@/lib/graphql/queries";
import { useCursorPagination } from "@/lib/hooks/useCursorPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import type { GetOrganizationsResult } from "@/lib/types";
import Link from "next/link";

export default function OrganizationsPage() {
  const pagination = useCursorPagination({ pageSize: 20 });

  const { data, isLoading, error } = useGraphQL<GetOrganizationsResult>(
    GET_ORGANIZATIONS,
    pagination.variables,
    { revalidateOnFocus: false }
  );

  const organizations = data?.organizations?.nodes || [];
  const totalCount = data?.organizations?.totalCount || 0;
  const pageInfo = data?.organizations?.pageInfo || { hasNextPage: false };
  const fullPageInfo = pagination.buildPageInfo(
    pageInfo.hasNextPage,
    pageInfo.startCursor,
    pageInfo.endCursor
  );

  if (isLoading && organizations.length === 0) {
    return <div className="p-8">Loading...</div>;
  }

  return (
    <div className="max-w-7xl mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">Organizations</h1>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
          Error: {error.message}
        </div>
      )}

      {organizations.length === 0 ? (
        <div className="text-stone-500 text-center py-12">No organizations found</div>
      ) : (
        <>
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 mb-6">
            {organizations.map((org) => (
              <Link
                key={org.id}
                href={`/admin/organizations/${org.id}`}
                className="bg-white border border-stone-200 rounded-lg p-6 hover:shadow-lg transition-shadow"
              >
                <h3 className="text-xl font-semibold mb-2">{org.name}</h3>
                {org.location && (
                  <p className="text-sm text-stone-500 mb-2">
                    {"\u{1F4CD}"} {org.location}
                  </p>
                )}
                {org.description && (
                  <p className="text-stone-600 text-sm line-clamp-3">{org.description}</p>
                )}
                {org.contactInfo?.website && (
                  <p className="text-amber-600 text-sm mt-2 truncate">{org.contactInfo.website}</p>
                )}
              </Link>
            ))}
          </div>

          <PaginationControls
            pageInfo={fullPageInfo}
            totalCount={totalCount}
            currentPage={pagination.currentPage}
            pageSize={pagination.pageSize}
            onNextPage={() => pagination.goToNextPage(pageInfo.endCursor ?? null)}
            onPreviousPage={pagination.goToPreviousPage}
            loading={isLoading}
          />
        </>
      )}
    </div>
  );
}
