"use client";

import { useState } from "react";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type {
  DiscoveryQuery,
  DiscoveryFilterRule,
  DiscoveryRun,
  DiscoveryRunResult,
  DiscoverySearchResult,
} from "@/lib/restate/types";

type Tab = "queries" | "filters" | "runs";

export default function DiscoveryPage() {
  const [activeTab, setActiveTab] = useState<Tab>("queries");
  const [isRunning, setIsRunning] = useState(false);
  const [runMessage, setRunMessage] = useState<string | null>(null);

  // Query data
  const { data: queriesData, isLoading: queriesLoading } = useRestate<{ queries: DiscoveryQuery[] }>(
    "Discovery", "list_queries", { include_inactive: true }, { revalidateOnFocus: false }
  );

  const { data: rulesData } = useRestate<{ rules: DiscoveryFilterRule[] }>(
    "Discovery", "list_filter_rules", { query_id: null }, { revalidateOnFocus: false }
  );

  const { data: runsData } = useRestate<{ runs: DiscoveryRun[] }>(
    "Discovery", "list_runs", { limit: 10 }, { revalidateOnFocus: false }
  );

  const queryList = queriesData?.queries || [];
  const ruleList = rulesData?.rules || [];
  const runList = runsData?.runs || [];

  const handleRunDiscovery = async () => {
    setIsRunning(true);
    setRunMessage(null);
    try {
      const r = await callService<DiscoverySearchResult>("Discovery", "run_search", {});
      setRunMessage(
        `Done! ${r.queries_run} queries, ${r.total_results} results, ${r.websites_created} websites created, ${r.websites_filtered} filtered`
      );
      invalidateService("Discovery");
    } catch (e: unknown) {
      setRunMessage(`Error: ${e instanceof Error ? e.message : "Unknown error"}`);
    } finally {
      setIsRunning(false);
    }
  };

  if (queriesLoading && queryList.length === 0) {
    return <AdminLoader label="Loading discovery..." />;
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Discovery</h1>
          <button
            onClick={handleRunDiscovery}
            disabled={isRunning}
            className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {isRunning ? "Running..." : "Run Discovery Now"}
          </button>
        </div>

      {runMessage && (
        <div
          className={`mb-6 px-4 py-3 rounded ${
            runMessage.startsWith("Error")
              ? "bg-red-50 border border-red-200 text-red-700"
              : "bg-green-50 border border-green-200 text-green-700"
          }`}
        >
          {runMessage}
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-1 mb-6 border-b border-stone-200">
        {(["queries", "filters", "runs"] as Tab[]).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab
                ? "border-amber-600 text-amber-700"
                : "border-transparent text-stone-500 hover:text-stone-700"
            }`}
          >
            {tab === "queries"
              ? `Queries (${queryList.length})`
              : tab === "filters"
                ? `Global Filters (${ruleList.length})`
                : `Runs (${runList.length})`}
          </button>
        ))}
      </div>

      {activeTab === "queries" && <QueriesTab queries={queryList} />}
      {activeTab === "filters" && <FiltersTab rules={ruleList} queryId={null} />}
      {activeTab === "runs" && <RunsTab runs={runList} />}
      </div>
    </div>
  );
}

// ============================================================================
// Queries Tab
// ============================================================================

function QueriesTab({ queries }: { queries: DiscoveryQuery[] }) {
  const [newQueryText, setNewQueryText] = useState("");
  const [newCategory, setNewCategory] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");
  const [editCategory, setEditCategory] = useState("");

  const handleCreate = async () => {
    if (!newQueryText.trim()) return;
    try {
      await callService("Discovery", "create_query", {
        query_text: newQueryText.trim(),
        category: newCategory.trim() || null,
      });
      setNewQueryText("");
      setNewCategory("");
      invalidateService("Discovery");
    } catch (e: unknown) {
      alert(`Failed to create query: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleUpdate = async (id: string) => {
    try {
      await callService("Discovery", "update_query", {
        id,
        query_text: editText.trim(),
        category: editCategory.trim() || null,
      });
      setEditingId(null);
      invalidateService("Discovery");
    } catch (e: unknown) {
      alert(`Failed to update: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleToggle = async (id: string, isActive: boolean) => {
    try {
      await callService("Discovery", "toggle_query", { id, is_active: !isActive });
      invalidateService("Discovery");
    } catch (e: unknown) {
      alert(`Failed to toggle: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this query? Results from previous runs are preserved.")) return;
    try {
      await callService("Discovery", "delete_query", { id });
      invalidateService("Discovery");
    } catch (e: unknown) {
      alert(`Failed to delete: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const categories = [...new Set(queries.map((q) => q.category).filter(Boolean))] as string[];

  return (
    <div>
      {/* Add new query */}
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <h3 className="text-sm font-medium text-stone-700 mb-3">Add Query</h3>
        <div className="flex gap-3">
          <input
            type="text"
            value={newQueryText}
            onChange={(e) => setNewQueryText(e.target.value)}
            placeholder="e.g. food bank {location}"
            className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
            onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          />
          <select
            value={newCategory}
            onChange={(e) => setNewCategory(e.target.value)}
            className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
          >
            <option value="">Category...</option>
            {categories.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
            <option value="services">services</option>
            <option value="professionals">professionals</option>
            <option value="businesses">businesses</option>
            <option value="opportunities">opportunities</option>
            <option value="events">events</option>
          </select>
          <button
            onClick={handleCreate}
            disabled={!newQueryText.trim()}
            className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50"
          >
            Add
          </button>
        </div>
      </div>

      {/* Query list grouped by category */}
      <div className="bg-white rounded-lg shadow overflow-hidden">
        <table className="min-w-full divide-y divide-stone-200">
          <thead className="bg-stone-50">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Query
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Category
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Status
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-stone-500 uppercase tracking-wider">
                Actions
              </th>
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-stone-200">
            {queries.map((query) => (
              <tr key={query.id} className={!query.is_active ? "opacity-50" : ""}>
                <td className="px-6 py-4">
                  {editingId === query.id ? (
                    <input
                      type="text"
                      value={editText}
                      onChange={(e) => setEditText(e.target.value)}
                      className="w-full px-2 py-1 border border-stone-300 rounded text-sm"
                      onKeyDown={(e) => e.key === "Enter" && handleUpdate(query.id)}
                    />
                  ) : (
                    <span className="text-sm text-stone-900 font-mono">{query.query_text}</span>
                  )}
                </td>
                <td className="px-6 py-4">
                  {editingId === query.id ? (
                    <input
                      type="text"
                      value={editCategory}
                      onChange={(e) => setEditCategory(e.target.value)}
                      className="w-24 px-2 py-1 border border-stone-300 rounded text-sm"
                    />
                  ) : (
                    <span className="text-xs px-2 py-1 bg-stone-100 text-stone-600 rounded">
                      {query.category || "none"}
                    </span>
                  )}
                </td>
                <td className="px-6 py-4">
                  <button
                    onClick={() => handleToggle(query.id, query.is_active)}
                    className={`text-xs px-2 py-1 rounded-full ${
                      query.is_active
                        ? "bg-green-100 text-green-700"
                        : "bg-stone-100 text-stone-500"
                    }`}
                  >
                    {query.is_active ? "active" : "inactive"}
                  </button>
                </td>
                <td className="px-6 py-4 text-right">
                  {editingId === query.id ? (
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => handleUpdate(query.id)}
                        className="text-xs font-medium text-green-600 hover:text-green-800"
                      >
                        Save
                      </button>
                      <button
                        onClick={() => setEditingId(null)}
                        className="text-xs font-medium text-stone-500 hover:text-stone-700"
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => {
                          setEditingId(query.id);
                          setEditText(query.query_text);
                          setEditCategory(query.category || "");
                        }}
                        className="text-xs font-medium text-stone-500 hover:text-stone-700"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => handleDelete(query.id)}
                        className="text-xs font-medium text-red-500 hover:text-red-700"
                      >
                        Delete
                      </button>
                    </div>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ============================================================================
// Filters Tab
// ============================================================================

function FiltersTab({
  rules,
  queryId,
}: {
  rules: DiscoveryFilterRule[];
  queryId: string | null;
}) {
  const [newRuleText, setNewRuleText] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");

  const handleCreate = async () => {
    if (!newRuleText.trim()) return;
    try {
      await callService("Discovery", "create_filter_rule", {
        query_id: queryId,
        rule_text: newRuleText.trim(),
      });
      setNewRuleText("");
      invalidateService("Discovery");
    } catch (e: unknown) {
      alert(`Failed to create rule: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleUpdate = async (id: string) => {
    try {
      await callService("Discovery", "update_filter_rule", {
        id,
        rule_text: editText.trim(),
      });
      setEditingId(null);
      invalidateService("Discovery");
    } catch (e: unknown) {
      alert(`Failed to update: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this filter rule?")) return;
    try {
      await callService("Discovery", "delete_filter_rule", { id });
      invalidateService("Discovery");
    } catch (e: unknown) {
      alert(`Failed to delete: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  return (
    <div>
      <div className="bg-white rounded-lg shadow p-4 mb-6">
        <h3 className="text-sm font-medium text-stone-700 mb-2">
          {queryId ? "Per-Query Filter Rules" : "Global Filter Rules"}
        </h3>
        <p className="text-xs text-stone-500 mb-3">
          {queryId
            ? "These rules apply only when this query runs. They override global rules if conflicting."
            : "These rules apply to ALL discovery queries. Write in plain text - AI evaluates each rule."}
        </p>
        <div className="flex gap-3">
          <input
            type="text"
            value={newRuleText}
            onChange={(e) => setNewRuleText(e.target.value)}
            placeholder='"Omit government websites"'
            className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
            onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          />
          <button
            onClick={handleCreate}
            disabled={!newRuleText.trim()}
            className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50"
          >
            Add Rule
          </button>
        </div>
      </div>

      <div className="space-y-2">
        {rules.map((rule, index) => (
          <div
            key={rule.id}
            className="bg-white rounded-lg shadow px-4 py-3 flex items-center gap-3"
          >
            <span className="text-xs text-stone-400 font-mono w-6">{index + 1}.</span>
            {editingId === rule.id ? (
              <>
                <input
                  type="text"
                  value={editText}
                  onChange={(e) => setEditText(e.target.value)}
                  className="flex-1 px-2 py-1 border border-stone-300 rounded text-sm"
                  onKeyDown={(e) => e.key === "Enter" && handleUpdate(rule.id)}
                />
                <button
                  onClick={() => handleUpdate(rule.id)}
                  className="text-xs font-medium text-green-600 hover:text-green-800"
                >
                  Save
                </button>
                <button
                  onClick={() => setEditingId(null)}
                  className="text-xs font-medium text-stone-500 hover:text-stone-700"
                >
                  Cancel
                </button>
              </>
            ) : (
              <>
                <span className="flex-1 text-sm text-stone-800">{rule.rule_text}</span>
                <button
                  onClick={() => {
                    setEditingId(rule.id);
                    setEditText(rule.rule_text);
                  }}
                  className="text-xs font-medium text-stone-500 hover:text-stone-700"
                >
                  Edit
                </button>
                <button
                  onClick={() => handleDelete(rule.id)}
                  className="text-xs font-medium text-red-500 hover:text-red-700"
                >
                  Delete
                </button>
              </>
            )}
          </div>
        ))}
        {rules.length === 0 && (
          <div className="text-stone-500 text-center py-8 text-sm">
            No filter rules yet. Add one above.
          </div>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Runs Tab
// ============================================================================

function RunsTab({ runs }: { runs: DiscoveryRun[] }) {
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);

  return (
    <div>
      <div className="bg-white rounded-lg shadow overflow-hidden mb-6">
        <table className="min-w-full divide-y divide-stone-200">
          <thead className="bg-stone-50">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Date
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Trigger
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-stone-500 uppercase tracking-wider">
                Queries
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-stone-500 uppercase tracking-wider">
                Results
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-stone-500 uppercase tracking-wider">
                Created
              </th>
              <th className="px-6 py-3 text-right text-xs font-medium text-stone-500 uppercase tracking-wider">
                Filtered
              </th>
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-stone-200">
            {runs.map((run) => (
              <tr
                key={run.id}
                onClick={() =>
                  setSelectedRunId(selectedRunId === run.id ? null : run.id)
                }
                className={`cursor-pointer hover:bg-stone-50 ${
                  selectedRunId === run.id ? "bg-amber-50" : ""
                }`}
              >
                <td className="px-6 py-4 text-sm text-stone-900 whitespace-nowrap">
                  {new Date(run.started_at).toLocaleString()}
                </td>
                <td className="px-6 py-4 whitespace-nowrap">
                  <span
                    className={`text-xs px-2 py-1 rounded-full ${
                      run.trigger_type === "manual"
                        ? "bg-blue-100 text-blue-700"
                        : "bg-stone-100 text-stone-600"
                    }`}
                  >
                    {run.trigger_type}
                  </span>
                </td>
                <td className="px-6 py-4 text-sm text-stone-600 text-right whitespace-nowrap">
                  {run.queries_executed}
                </td>
                <td className="px-6 py-4 text-sm text-stone-600 text-right whitespace-nowrap">
                  {run.total_results}
                </td>
                <td className="px-6 py-4 text-sm text-green-600 font-medium text-right whitespace-nowrap">
                  {run.websites_created}
                </td>
                <td className="px-6 py-4 text-sm text-red-600 font-medium text-right whitespace-nowrap">
                  {run.websites_filtered}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {selectedRunId && <RunResultsDetail runId={selectedRunId} />}

      {runs.length === 0 && (
        <div className="text-stone-500 text-center py-12 text-sm">
          No discovery runs yet. Click &quot;Run Discovery Now&quot; to start.
        </div>
      )}
    </div>
  );
}

// ============================================================================
// Run Results Detail (expanded view)
// ============================================================================

function RunResultsDetail({ runId }: { runId: string }) {
  const { data, isLoading } = useRestate<{ results: DiscoveryRunResult[] }>(
    "Discovery", "run_results", { run_id: runId }, { revalidateOnFocus: false }
  );

  const results = data?.results || [];

  if (isLoading) {
    return (
      <div className="text-center py-4 text-stone-500 text-sm">Loading results...</div>
    );
  }

  const passed = results.filter((r) => r.filter_result === "passed");
  const filtered = results.filter((r) => r.filter_result === "filtered");

  return (
    <div className="bg-white rounded-lg shadow p-4">
      <h3 className="text-sm font-medium text-stone-700 mb-3">
        Run Results ({results.length} total: {passed.length} passed, {filtered.length} filtered)
      </h3>
      <div className="space-y-2 max-h-96 overflow-y-auto">
        {results.map((result) => (
          <div
            key={result.id}
            className={`px-3 py-2 rounded border text-sm ${
              result.filter_result === "passed"
                ? "border-green-200 bg-green-50"
                : result.filter_result === "filtered"
                  ? "border-red-200 bg-red-50"
                  : "border-stone-200 bg-stone-50"
            }`}
          >
            <div className="flex items-center justify-between">
              <div>
                <span className="font-medium text-stone-900">{result.domain}</span>
                {result.title && (
                  <span className="text-stone-500 ml-2">- {result.title}</span>
                )}
              </div>
              <span
                className={`text-xs px-2 py-0.5 rounded-full ${
                  result.filter_result === "passed"
                    ? "bg-green-200 text-green-800"
                    : "bg-red-200 text-red-800"
                }`}
              >
                {result.filter_result}
              </span>
            </div>
            {result.filter_reason && (
              <div className="text-xs text-stone-500 mt-1">{result.filter_reason}</div>
            )}
            {result.relevance_score != null && (
              <div className="text-xs text-stone-400 mt-1">
                Score: {result.relevance_score.toFixed(2)}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
