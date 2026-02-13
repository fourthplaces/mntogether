"use client";

import { Suspense, useState } from "react";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { SearchQueryResult, SearchQueryListResult } from "@/lib/restate/types";

export default function SearchQueriesPage() {
  return (
    <Suspense fallback={<AdminLoader label="Loading search queries..." />}>
      <SearchQueriesContent />
    </Suspense>
  );
}

function SearchQueriesContent() {
  const { data, isLoading } = useRestate<SearchQueryListResult>(
    "Websites", "list_search_queries", {}, { revalidateOnFocus: false }
  );
  const [showAdd, setShowAdd] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [runningDiscovery, setRunningDiscovery] = useState(false);

  const handleToggle = async (query: SearchQueryResult) => {
    try {
      await callService("Websites", "toggle_search_query", { id: query.id });
      invalidateService("Websites");
    } catch (err) {
      console.error("Failed to toggle query:", err);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await callService("Websites", "delete_search_query", { id });
      invalidateService("Websites");
    } catch (err) {
      console.error("Failed to delete query:", err);
    }
  };

  const handleRunDiscovery = async () => {
    setRunningDiscovery(true);
    try {
      await callService("Websites", "run_scheduled_discovery", {});
    } catch (err) {
      console.error("Failed to run discovery:", err);
    } finally {
      setRunningDiscovery(false);
    }
  };

  if (isLoading) {
    return <AdminLoader label="Loading search queries..." />;
  }

  const queries = data?.queries || [];

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-4xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-3xl font-bold text-stone-900">Search Queries</h1>
            <p className="text-sm text-stone-500 mt-1">
              Tavily search queries used for website discovery
            </p>
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleRunDiscovery}
              disabled={runningDiscovery}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-100 text-stone-700 hover:bg-stone-200 disabled:opacity-50 transition-colors"
            >
              {runningDiscovery ? "Running..." : "Run Discovery"}
            </button>
            <button
              onClick={() => setShowAdd(!showAdd)}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors"
            >
              + Add Query
            </button>
          </div>
        </div>

        {showAdd && (
          <AddQueryForm onClose={() => setShowAdd(false)} />
        )}

        <div className="space-y-2">
          {queries.map((query) => (
            <div key={query.id}>
              {editingId === query.id ? (
                <EditQueryForm
                  query={query}
                  onClose={() => setEditingId(null)}
                />
              ) : (
                <div className="bg-white rounded-lg shadow px-4 py-3 flex items-center justify-between">
                  <div className="flex items-center gap-3 flex-1 min-w-0">
                    <button
                      onClick={() => handleToggle(query)}
                      className={`shrink-0 w-9 h-5 rounded-full transition-colors ${
                        query.is_active ? "bg-green-500" : "bg-stone-300"
                      }`}
                    >
                      <div
                        className={`w-4 h-4 bg-white rounded-full shadow transition-transform ${
                          query.is_active ? "translate-x-4" : "translate-x-0.5"
                        }`}
                      />
                    </button>
                    <span
                      className={`text-sm font-medium truncate ${
                        query.is_active ? "text-stone-900" : "text-stone-400"
                      }`}
                    >
                      {query.query_text}
                    </span>
                  </div>
                  <div className="flex items-center gap-1 shrink-0 ml-3">
                    <button
                      onClick={() => setEditingId(query.id)}
                      className="px-2 py-1 text-xs text-stone-500 hover:text-amber-700 hover:bg-amber-50 rounded transition-colors"
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDelete(query.id)}
                      className="px-2 py-1 text-xs text-stone-500 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
                    >
                      Delete
                    </button>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>

        {queries.length === 0 && !showAdd && (
          <div className="text-stone-500 text-center py-12">
            No search queries yet. Add one to start discovering websites.
          </div>
        )}
      </div>
    </div>
  );
}

// =============================================================================
// Add Query Form
// =============================================================================

function AddQueryForm({ onClose }: { onClose: () => void }) {
  const [queryText, setQueryText] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!queryText.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await callService("Websites", "create_search_query", {
        query_text: queryText.trim(),
      });
      invalidateService("Websites");
      onClose();
    } catch (err: any) {
      setError(err.message || "Failed to create query");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="bg-white rounded-lg shadow px-4 py-4 mb-4 space-y-3"
    >
      <div className="text-sm font-medium text-stone-700">New Search Query</div>
      <input
        type="text"
        value={queryText}
        onChange={(e) => setQueryText(e.target.value)}
        placeholder='e.g. "Minnesota community resources food assistance"'
        className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        autoFocus
        disabled={loading}
      />
      <div className="flex items-center gap-2">
        <button
          type="submit"
          disabled={loading || !queryText.trim()}
          className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
        >
          {loading ? "Creating..." : "Create"}
        </button>
        <button
          type="button"
          onClick={onClose}
          className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
        >
          Cancel
        </button>
        {error && <span className="text-red-600 text-sm">{error}</span>}
      </div>
    </form>
  );
}

// =============================================================================
// Edit Query Form
// =============================================================================

function EditQueryForm({
  query,
  onClose,
}: {
  query: SearchQueryResult;
  onClose: () => void;
}) {
  const [queryText, setQueryText] = useState(query.query_text);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!queryText.trim()) return;
    setLoading(true);
    setError(null);
    try {
      await callService("Websites", "update_search_query", {
        id: query.id,
        query_text: queryText.trim(),
      });
      invalidateService("Websites");
      onClose();
    } catch (err: any) {
      setError(err.message || "Failed to update query");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="bg-white rounded-lg shadow px-4 py-4 space-y-3 border-2 border-amber-200"
    >
      <input
        type="text"
        value={queryText}
        onChange={(e) => setQueryText(e.target.value)}
        className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        autoFocus
        disabled={loading}
      />
      <div className="flex items-center gap-2">
        <button
          type="submit"
          disabled={loading || !queryText.trim()}
          className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
        >
          {loading ? "Saving..." : "Save"}
        </button>
        <button
          type="button"
          onClick={onClose}
          className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
        >
          Cancel
        </button>
        {error && <span className="text-red-600 text-sm">{error}</span>}
      </div>
    </form>
  );
}
