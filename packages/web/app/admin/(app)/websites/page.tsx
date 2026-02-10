"use client";

import { Suspense, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import type { WebsiteList, WebsiteResult } from "@/lib/restate/types";

export default function WebsitesPage() {
  return (
    <Suspense fallback={<AdminLoader label="Loading websites..." />}>
      <WebsitesContent />
    </Suspense>
  );
}

function WebsitesContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const statusFilter = searchParams.get("status");
  const pagination = useOffsetPagination({ pageSize: 20 });

  const setStatusFilter = (status: string | null) => {
    const params = new URLSearchParams(searchParams.toString());
    if (status) {
      params.set("status", status);
    } else {
      params.delete("status");
    }
    router.replace(`/admin/websites?${params.toString()}`);
    pagination.reset();
  };

  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      pagination.reset();
    }, 300);
    return () => clearTimeout(timer);
  }, [search]);

  const [showAddForm, setShowAddForm] = useState(false);
  const [addUrl, setAddUrl] = useState("");
  const [addLoading, setAddLoading] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);

  const handleAddWebsite = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!addUrl.trim()) return;

    setAddLoading(true);
    setAddError(null);
    try {
      const result = await callService<WebsiteResult>("Websites", "submit", { url: addUrl.trim() });
      invalidateService("Websites");
      setAddUrl("");
      setShowAddForm(false);
      if (result?.id) {
        router.push(`/admin/websites/${result.id}`);
      }
    } catch (err: any) {
      setAddError(err.message || "Failed to add website");
    } finally {
      setAddLoading(false);
    }
  };

  const { data, isLoading, error } = useRestate<WebsiteList>(
    "Websites", "list",
    {
      ...pagination.variables,
      status: statusFilter,
      search: debouncedSearch || undefined,
    },
    { revalidateOnFocus: false }
  );

  const websites = data?.websites || [];
  const totalCount = data?.total_count || 0;
  const hasNextPage = data?.has_next_page || false;
  const pageInfo = pagination.buildPageInfo(hasNextPage);

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
          <div className="flex gap-2 items-center">
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search domains..."
              className="px-3 py-1.5 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent w-48"
            />
            {["all", "pending_review", "approved", "rejected"].map((status) => (
              <button
                key={status}
                onClick={() => setStatusFilter(status === "all" ? null : status)}
                className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                  (status === "all" && !statusFilter) || statusFilter === status
                    ? "bg-amber-600 text-white"
                    : "bg-stone-100 text-stone-700 hover:bg-stone-200"
                }`}
              >
                {status === "all" ? "All" : status.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())}
              </button>
            ))}
            <button
              onClick={() => setShowAddForm(!showAddForm)}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors ml-2"
            >
              + Add Website
            </button>
          </div>
        </div>

        {showAddForm && (
          <form onSubmit={handleAddWebsite} className="bg-white rounded-lg shadow px-4 py-3 mb-6 flex items-center gap-3">
            <input
              type="text"
              value={addUrl}
              onChange={(e) => setAddUrl(e.target.value)}
              placeholder="Enter URL or domain (e.g. example.com)"
              className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
              autoFocus
              disabled={addLoading}
            />
            <button
              type="submit"
              disabled={addLoading || !addUrl.trim()}
              className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {addLoading ? "Adding..." : "Add"}
            </button>
            <button
              type="button"
              onClick={() => { setShowAddForm(false); setAddUrl(""); setAddError(null); }}
              className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
            >
              Cancel
            </button>
            {addError && (
              <span className="text-red-600 text-sm">{addError}</span>
            )}
          </form>
        )}

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
                    Posts
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
                        {website.status.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-600">
                      {website.post_count || 0}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-500 text-sm">
                      {website.last_crawled_at
                        ? new Date(website.last_crawled_at).toLocaleDateString()
                        : "Never"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <PaginationControls
            pageInfo={pageInfo}
            totalCount={totalCount}
            currentPage={pagination.currentPage}
            pageSize={pagination.pageSize}
            onNextPage={pagination.goToNextPage}
            onPreviousPage={pagination.goToPreviousPage}
            loading={isLoading}
          />
        </>
      )}
      </div>
    </div>
  );
}
