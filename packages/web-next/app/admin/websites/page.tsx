"use client";

import Link from "next/link";
import { useGraphQL, graphqlMutateClient, invalidateAllMatchingQuery } from "@/lib/graphql/client";
import { GET_ALL_WEBSITES } from "@/lib/graphql/queries";
import { APPROVE_WEBSITE, REJECT_WEBSITE, CRAWL_WEBSITE } from "@/lib/graphql/mutations";
import { useCursorPagination } from "@/lib/hooks/useCursorPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import type { Website, GetWebsitesResult } from "@/lib/types";
import { useState } from "react";

export default function WebsitesPage() {
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const pagination = useCursorPagination({ pageSize: 20 });

  const { data, isLoading, error, mutate: refetch } = useGraphQL<GetWebsitesResult>(
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

  const handleApprove = async (websiteId: string) => {
    if (!confirm("Approve this website for crawling?")) return;

    setActionInProgress(websiteId);
    try {
      await graphqlMutateClient(APPROVE_WEBSITE, { websiteId });
      invalidateAllMatchingQuery(GET_ALL_WEBSITES);
      refetch();
    } catch (err) {
      console.error("Failed to approve:", err);
      alert("Failed to approve website");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async (websiteId: string) => {
    const reason = prompt("Reason for rejection:");
    if (reason === null) return;

    setActionInProgress(websiteId);
    try {
      await graphqlMutateClient(REJECT_WEBSITE, { websiteId, reason: reason || "Rejected" });
      invalidateAllMatchingQuery(GET_ALL_WEBSITES);
      refetch();
    } catch (err) {
      console.error("Failed to reject:", err);
      alert("Failed to reject website");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleCrawl = async (websiteId: string) => {
    setActionInProgress(websiteId);
    try {
      await graphqlMutateClient(CRAWL_WEBSITE, { websiteId });
      refetch();
    } catch (err) {
      console.error("Failed to start crawl:", err);
      alert("Failed to start crawl");
    } finally {
      setActionInProgress(null);
    }
  };

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
    return <div className="p-8">Loading...</div>;
  }

  return (
    <div className="max-w-7xl mx-auto p-8">
      <div className="flex items-center justify-between mb-8">
        <h1 className="text-3xl font-bold">Websites</h1>
        <div className="flex gap-2">
          {["all", "pending_review", "approved", "rejected"].map((status) => (
            <button
              key={status}
              onClick={() => {
                setStatusFilter(status === "all" ? null : status);
                pagination.reset();
              }}
              className={`px-3 py-1 rounded text-sm ${
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
                  <th className="px-6 py-3 text-right text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-stone-200">
                {websites.map((website) => (
                  <tr key={website.id} className="hover:bg-stone-50">
                    <td className="px-6 py-4 whitespace-nowrap">
                      <Link
                        href={`/admin/websites/${website.id}`}
                        className="text-amber-600 hover:text-amber-800 font-medium"
                      >
                        {website.domain}
                      </Link>
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
                    <td className="px-6 py-4 whitespace-nowrap text-right text-sm">
                      <div className="flex justify-end gap-2">
                        {website.status === "pending_review" && (
                          <>
                            <button
                              onClick={() => handleApprove(website.id)}
                              disabled={actionInProgress === website.id}
                              className="text-green-600 hover:text-green-800 disabled:opacity-50"
                            >
                              Approve
                            </button>
                            <button
                              onClick={() => handleReject(website.id)}
                              disabled={actionInProgress === website.id}
                              className="text-red-600 hover:text-red-800 disabled:opacity-50"
                            >
                              Reject
                            </button>
                          </>
                        )}
                        {website.status === "approved" && (
                          <button
                            onClick={() => handleCrawl(website.id)}
                            disabled={actionInProgress === website.id}
                            className="text-amber-600 hover:text-amber-800 disabled:opacity-50"
                          >
                            Crawl
                          </button>
                        )}
                        <Link
                          href={`/admin/websites/${website.id}`}
                          className="text-stone-600 hover:text-stone-800"
                        >
                          Details
                        </Link>
                      </div>
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
  );
}
