"use client";

import { useGraphQL } from "@/lib/graphql/client";
import { GET_RESOURCES } from "@/lib/graphql/queries";
import { useCursorPagination } from "@/lib/hooks/useCursorPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import type { GetResourcesResult } from "@/lib/types";
import Link from "next/link";

export default function ResourcesPage() {
  const pagination = useCursorPagination({ pageSize: 20 });

  const { data, isLoading, error } = useGraphQL<GetResourcesResult>(
    GET_RESOURCES,
    {
      ...pagination.variables,
      status: "PENDING",
    },
    { revalidateOnFocus: false }
  );

  const resources = data?.resources?.nodes || [];
  const totalCount = data?.resources?.totalCount || 0;
  const pageInfo = data?.resources?.pageInfo || { hasNextPage: false };
  const fullPageInfo = pagination.buildPageInfo(
    pageInfo.hasNextPage,
    pageInfo.startCursor,
    pageInfo.endCursor
  );

  if (isLoading && resources.length === 0) {
    return <div className="p-8">Loading...</div>;
  }

  return (
    <div className="max-w-7xl mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">Resources</h1>

      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
          Error: {error.message}
        </div>
      )}

      {resources.length === 0 ? (
        <div className="text-stone-500 text-center py-12">No pending resources to review</div>
      ) : (
        <>
          <div className="grid gap-4 mb-6">
            {resources.map((resource) => (
              <div
                key={resource.id}
                className="bg-white border border-stone-200 rounded-lg p-6 hover:shadow-lg transition-shadow"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <h3 className="text-xl font-semibold mb-2">{resource.title}</h3>
                    {resource.organizationName && (
                      <p className="text-sm text-stone-600 mb-2">{resource.organizationName}</p>
                    )}
                    {resource.location && (
                      <p className="text-sm text-stone-500 mb-2">
                        {"\u{1F4CD}"} {resource.location}
                      </p>
                    )}
                    <p className="text-stone-700 line-clamp-3">{resource.content}</p>
                  </div>
                  <Link
                    href={`/admin/resources/${resource.id}`}
                    className="px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700"
                  >
                    View
                  </Link>
                </div>
              </div>
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
