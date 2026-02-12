"use client";

import { Suspense, useEffect, useState, useMemo } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import type { SourceListResult, SourceResult, LightCrawlAllResult } from "@/lib/restate/types";

export default function SourcesPage() {
  return (
    <Suspense fallback={<AdminLoader label="Loading sources..." />}>
      <SourcesContent />
    </Suspense>
  );
}

const SOURCE_TYPE_LABELS: Record<string, string> = {
  website: "Website",
  instagram: "Instagram",
  facebook: "Facebook",
  tiktok: "TikTok",
  x: "X (Twitter)",
};

function SourcesContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const statusFilter = searchParams.get("status");
  const typeFilter = searchParams.get("type");
  const pagination = useOffsetPagination({ pageSize: 20 });

  const setFilter = (key: string, value: string | null) => {
    const params = new URLSearchParams(searchParams.toString());
    if (value) {
      params.set(key, value);
    } else {
      params.delete(key);
    }
    router.replace(`/admin/sources?${params.toString()}`);
    pagination.reset();
  };

  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [aiSearch, setAiSearch] = useState(false);
  const [aiResults, setAiResults] = useState<SourceListResult | null>(null);
  const [aiLoading, setAiLoading] = useState(false);

  useEffect(() => {
    if (aiSearch) return; // skip debounce when AI search is on
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      pagination.reset();
    }, 300);
    return () => clearTimeout(timer);
  }, [search, aiSearch]);

  const runAiSearch = async () => {
    if (!search.trim()) return;
    setAiLoading(true);
    try {
      const result = await callService<SourceListResult>("Sources", "search_by_content", {
        query: search.trim(),
        limit: 100,
      });
      setAiResults(result);
    } catch (err: any) {
      setAiResults(null);
    } finally {
      setAiLoading(false);
    }
  };

  // Clear AI results when toggling off or clearing search
  useEffect(() => {
    if (!aiSearch) setAiResults(null);
  }, [aiSearch]);

  const [showAddForm, setShowAddForm] = useState(false);
  const [addUrl, setAddUrl] = useState("");
  const [addLoading, setAddLoading] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [lightCrawlLoading, setLightCrawlLoading] = useState(false);
  const [lightCrawlResult, setLightCrawlResult] = useState<string | null>(null);

  const handleLightCrawlAll = async () => {
    setLightCrawlLoading(true);
    setLightCrawlResult(null);
    try {
      const result = await callService<LightCrawlAllResult>("Sources", "light_crawl_all", {});
      setLightCrawlResult(`Queued ${result.sources_queued} sources for light crawl`);
    } catch (err: any) {
      setLightCrawlResult(`Error: ${err.message || "Failed to start light crawl"}`);
    } finally {
      setLightCrawlLoading(false);
    }
  };

  const handleAddWebsite = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!addUrl.trim()) return;

    setAddLoading(true);
    setAddError(null);
    try {
      const result = await callService<SourceResult>("Sources", "submit_website", { url: addUrl.trim() });
      invalidateService("Sources");
      setAddUrl("");
      setShowAddForm(false);
      if (result?.id) {
        router.push(`/admin/sources/${result.id}`);
      }
    } catch (err: any) {
      setAddError(err.message || "Failed to add website");
    } finally {
      setAddLoading(false);
    }
  };

  const { data, isLoading, error } = useRestate<SourceListResult>(
    "Sources", "list",
    {
      ...pagination.variables,
      status: statusFilter,
      source_type: typeFilter,
      search: (!aiSearch && debouncedSearch) || undefined,
    },
    { revalidateOnFocus: false }
  );

  const activeData = (aiSearch && aiResults) ? aiResults : data;
  const sources = activeData?.sources || [];
  const totalCount = activeData?.total_count || 0;
  const hasNextPage = activeData?.has_next_page || false;
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

  const getTypeColor = (sourceType: string) => {
    switch (sourceType) {
      case "website":
        return "bg-blue-100 text-blue-800";
      case "instagram":
        return "bg-purple-100 text-purple-800";
      case "facebook":
        return "bg-indigo-100 text-indigo-800";
      case "tiktok":
        return "bg-pink-100 text-pink-800";
      default:
        return "bg-stone-100 text-stone-800";
    }
  };

  if ((isLoading || aiLoading) && sources.length === 0) {
    return <AdminLoader label={aiLoading ? "Searching content..." : "Loading sources..."} />;
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Sources</h1>
          <div className="flex gap-2 items-center">
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && aiSearch) runAiSearch();
              }}
              placeholder={aiSearch ? "AI search (press Enter)..." : "Search..."}
              className="px-3 py-1.5 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent w-48"
            />
            <button
              onClick={() => setAiSearch(!aiSearch)}
              className={`px-2 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                aiSearch
                  ? "bg-violet-600 text-white"
                  : "bg-stone-100 text-stone-500 hover:bg-stone-200"
              }`}
              title="Toggle AI semantic search"
            >
              AI
            </button>
            {/* Type filter */}
            {["all", "website", "instagram", "facebook"].map((type) => (
              <button
                key={type}
                onClick={() => setFilter("type", type === "all" ? null : type)}
                className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                  (type === "all" && !typeFilter) || typeFilter === type
                    ? "bg-amber-600 text-white"
                    : "bg-stone-100 text-stone-700 hover:bg-stone-200"
                }`}
              >
                {type === "all" ? "All Types" : SOURCE_TYPE_LABELS[type] || type}
              </button>
            ))}
            <span className="text-stone-300">|</span>
            {/* Status filter */}
            {["all", "pending_review", "approved", "rejected"].map((status) => (
              <button
                key={status}
                onClick={() => setFilter("status", status === "all" ? null : status)}
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
              onClick={handleLightCrawlAll}
              disabled={lightCrawlLoading}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-violet-600 text-white hover:bg-violet-700 disabled:opacity-50 transition-colors ml-2"
              title="Light crawl all uncrawled sources (5 pages, depth 1)"
            >
              {lightCrawlLoading ? "Queuing..." : "Light Crawl All"}
            </button>
            <button
              onClick={() => setShowAddForm(!showAddForm)}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors"
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

        {lightCrawlResult && (
          <div className="bg-violet-50 border border-violet-200 text-violet-700 px-4 py-3 rounded mb-6 flex items-center justify-between">
            {lightCrawlResult}
            <button onClick={() => setLightCrawlResult(null)} className="text-violet-400 hover:text-violet-600 text-sm ml-4">dismiss</button>
          </div>
        )}

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            Error: {error.message}
          </div>
        )}

        {sources.length === 0 ? (
          <div className="text-stone-500 text-center py-12">No sources found</div>
        ) : (
        <>
          <div className="bg-white rounded-lg shadow overflow-hidden mb-6">
            <table className="min-w-full divide-y divide-stone-200">
              <thead className="bg-stone-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Type
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Identifier
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Organization
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
                {sources.map((source) => (
                  <tr
                    key={source.id}
                    onClick={() => router.push(`/admin/sources/${source.id}`)}
                    className="hover:bg-stone-50 cursor-pointer"
                  >
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${getTypeColor(source.source_type)}`}>
                        {SOURCE_TYPE_LABELS[source.source_type] || source.source_type}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap font-medium text-stone-900">
                      {source.identifier}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-600">
                      {source.organization_name || (
                        <span className="text-stone-300">{"\u2014"}</span>
                      )}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`px-2 py-1 text-xs rounded-full ${getStatusColor(source.status)}`}>
                        {source.status.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-600">
                      {source.post_count || 0}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-500 text-sm">
                      {source.last_scraped_at
                        ? new Date(source.last_scraped_at).toLocaleDateString()
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
