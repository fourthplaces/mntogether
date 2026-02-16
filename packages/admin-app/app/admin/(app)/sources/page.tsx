"use client";

import { useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useOffsetPagination } from "@/lib/hooks/useOffsetPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import {
  SourcesListQuery,
  SearchSourcesByContentQuery,
  SubmitWebsiteMutation,
  LightCrawlAllMutation,
} from "@/lib/graphql/sources";

export default function SourcesPage() {
  return <SourcesContent />;
}

const SOURCE_TYPE_LABELS: Record<string, string> = {
  website: "Website",
  instagram: "Instagram",
  facebook: "Facebook",
  tiktok: "TikTok",
  x: "X (Twitter)",
  newsletter: "Newsletter",
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
  const [aiSearchQuery, setAiSearchQuery] = useState<string | null>(null);

  useEffect(() => {
    if (aiSearch) return;
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      pagination.reset();
    }, 300);
    return () => clearTimeout(timer);
  }, [search, aiSearch]);

  const runAiSearch = () => {
    if (!search.trim()) return;
    setAiSearchQuery(search.trim());
  };

  useEffect(() => {
    if (!aiSearch) setAiSearchQuery(null);
  }, [aiSearch]);

  const [showAddForm, setShowAddForm] = useState(false);
  const [addUrl, setAddUrl] = useState("");
  const [addError, setAddError] = useState<string | null>(null);
  const [lightCrawlResult, setLightCrawlResult] = useState<string | null>(null);

  const [{ fetching: addLoading }, submitWebsite] = useMutation(SubmitWebsiteMutation);
  const [{ fetching: lightCrawlLoading }, lightCrawlAll] = useMutation(LightCrawlAllMutation);

  const mutationContext = { additionalTypenames: ["Source", "SourceConnection"] };

  const handleLightCrawlAll = async () => {
    setLightCrawlResult(null);
    const result = await lightCrawlAll({}, mutationContext);
    if (result.error) {
      setLightCrawlResult(`Error: ${result.error.message || "Failed to start light crawl"}`);
    } else {
      setLightCrawlResult(`Queued ${result.data?.lightCrawlAll?.sourcesQueued} sources for light crawl`);
    }
  };

  const handleAddWebsite = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!addUrl.trim()) return;

    setAddError(null);
    const result = await submitWebsite({ url: addUrl.trim() }, mutationContext);
    if (result.error) {
      setAddError(result.error.message || "Failed to add website");
    } else {
      setAddUrl("");
      setShowAddForm(false);
      if (result.data?.submitWebsite?.id) {
        router.push(`/admin/sources/${result.data.submitWebsite.id}`);
      }
    }
  };

  const [{ data, fetching: isLoading, error }] = useQuery({
    query: SourcesListQuery,
    variables: {
      status: statusFilter,
      sourceType: typeFilter,
      search: (!aiSearch && debouncedSearch) || null,
      limit: pagination.variables.first ?? 20,
      offset: pagination.variables.offset ?? 0,
    },
  });

  const [{ data: aiData, fetching: aiLoading }] = useQuery({
    query: SearchSourcesByContentQuery,
    variables: { query: aiSearchQuery || "", limit: 100 },
    pause: !aiSearchQuery,
  });

  const activeData = (aiSearch && aiData?.searchSourcesByContent) ? aiData.searchSourcesByContent : data?.sources;
  const sources = activeData?.sources || [];
  const totalCount = activeData?.totalCount || 0;
  const hasNextPage = activeData?.hasNextPage || false;
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
      case "newsletter":
        return "bg-emerald-100 text-emerald-800";
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
            {["all", "website", "instagram", "facebook", "newsletter"].map((type) => (
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
                      <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${getTypeColor(source.sourceType)}`}>
                        {SOURCE_TYPE_LABELS[source.sourceType] || source.sourceType}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap font-medium text-stone-900">
                      {source.identifier}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-600">
                      {source.organizationName || (
                        <span className="text-stone-300">{"\u2014"}</span>
                      )}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`px-2 py-1 text-xs rounded-full ${getStatusColor(source.status)}`}>
                        {source.status.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-600">
                      {source.postCount || 0}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-500 text-sm">
                      {source.lastScrapedAt
                        ? new Date(source.lastScrapedAt).toLocaleDateString()
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
