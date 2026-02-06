"use client";

import { useRouter } from "next/navigation";
import { useGraphQL } from "@/lib/graphql/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { GET_ALL_WEBSITES } from "@/lib/graphql/queries";
import { useCursorPagination } from "@/lib/hooks/useCursorPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import type { Website, GetWebsitesResult } from "@/lib/types";
import { useState } from "react";

export default function WebsitesPage() {
  const router = useRouter();
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const pagination = useCursorPagination({ pageSize: 20 });

  const { data, isLoading, error } = useGraphQL<GetWebsitesResult>(
    GET_ALL_WEBSITES,
    {
      ...pagination.variables,
      status: statusFilter,
    },
    { revalidateOnFocus: false }
  );

  const websites = data?.websites?.nodes || [];
  const totalCount = data?.websites?.totalCount || 0;
  const pageInfo = data?.websites?.pageInfo || { hasNextPage: false };
  const fullPageInfo = pagination.buildPageInfo(
    pageInfo.hasNextPage,
    pageInfo.startCursor,
    pageInfo.endCursor
  );

  const getStatusColor = (status: string) => {
    switch (status?.toLowerCase()) {
      case "approved":
        return "bg-green-100 text-green-800";
      case "pending_review":
      case "pending":
        return "bg-yellow-100 text-yellow-800";
      case "rejected":
        return "bg-red-100 text-red-800";
      case "suspended":
        return "bg-gray-100 text-gray-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  if (isLoading && websites.length === 0) {
    return <AdminLoader label="Loading websites..." />;
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Websites</h1>
          <div className="flex gap-2">
            {["all", "pending_review", "approved", "rejected"].map((status) => (
              <button
                key={status}
                onClick={() => {
                  setStatusFilter(status === "all" ? null : status);
                  pagination.reset();
                }}
                className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                  (status === "all" && !statusFilter) || statusFilter === status
                    ? "bg-amber-600 text-white"
                    : "bg-stone-100 text-stone-700 hover:bg-stone-200"
                }`}
              >
                {status === "all" ? "All" : status.replace("_", " ")}
              </button>
            ))}
          </div>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            Error: {error.message}
          </div>
        )}

        {websites.length === 0 ? (
          <div className="text-stone-500 text-center py-12">No websites found</div>
        ) : (
        <>
          <div className="bg-white rounded-lg shadow overflow-hidden mb-6">
            <table className="min-w-full divide-y divide-stone-200">
              <thead className="bg-stone-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Domain
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Listings
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Last Scraped
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-stone-200">
                {websites.map((website) => (
                  <tr
                    key={website.id}
                    onClick={() => router.push(`/admin/websites/${website.id}`)}
                    className="hover:bg-stone-50 cursor-pointer"
                  >
                    <td className="px-6 py-4 whitespace-nowrap font-medium text-stone-900">
                      {website.domain}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`px-2 py-1 text-xs rounded-full ${getStatusColor(website.status)}`}>
                        {website.status}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-600">
                      {website.listingsCount || 0}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-500 text-sm">
                      {website.lastScrapedAt
                        ? new Date(website.lastScrapedAt).toLocaleDateString()
                        : "Never"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
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
    </div>
  );
}
