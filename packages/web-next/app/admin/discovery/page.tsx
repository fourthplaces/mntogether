"use client";

import { useState } from "react";
import { useGraphQL, graphqlMutateClient, invalidateAllMatchingQuery } from "@/lib/graphql/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  GET_DISCOVERY_QUERIES,
  GET_DISCOVERY_FILTER_RULES,
  GET_DISCOVERY_RUNS,
  GET_DISCOVERY_RUN_RESULTS,
} from "@/lib/graphql/queries";
import {
  RUN_DISCOVERY_SEARCH,
  CREATE_DISCOVERY_QUERY,
  UPDATE_DISCOVERY_QUERY,
  TOGGLE_DISCOVERY_QUERY,
  DELETE_DISCOVERY_QUERY,
  CREATE_DISCOVERY_FILTER_RULE,
  UPDATE_DISCOVERY_FILTER_RULE,
  DELETE_DISCOVERY_FILTER_RULE,
} from "@/lib/graphql/mutations";

interface DiscoveryQuery {
  id: string;
  queryText: string;
  category: string | null;
  isActive: boolean;
  createdAt: string;
}

interface FilterRule {
  id: string;
  queryId: string | null;
  ruleText: string;
  sortOrder: number;
  isActive: boolean;
}

interface DiscoveryRun {
  id: string;
  queriesExecuted: number;
  totalResults: number;
  websitesCreated: number;
  websitesFiltered: number;
  startedAt: string;
  completedAt: string | null;
  triggerType: string;
}

interface RunResult {
  id: string;
  runId: string;
  queryId: string;
  domain: string;
  url: string;
  title: string | null;
  snippet: string | null;
  relevanceScore: number | null;
  filterResult: string;
  filterReason: string | null;
  websiteId: string | null;
  discoveredAt: string;
}

type Tab = "queries" | "filters" | "runs";

export default function DiscoveryPage() {
  const [activeTab, setActiveTab] = useState<Tab>("queries");
  const [isRunning, setIsRunning] = useState(false);
  const [runMessage, setRunMessage] = useState<string | null>(null);

  // Query data
  const { data: queriesData, isLoading: queriesLoading } = useGraphQL<{
    discoveryQueries: DiscoveryQuery[];
  }>(GET_DISCOVERY_QUERIES, { includeInactive: true }, { revalidateOnFocus: false });

  const { data: globalRulesData } = useGraphQL<{
    discoveryFilterRules: FilterRule[];
  }>(GET_DISCOVERY_FILTER_RULES, { queryId: null }, { revalidateOnFocus: false });

  const { data: runsData } = useGraphQL<{
    discoveryRuns: DiscoveryRun[];
  }>(GET_DISCOVERY_RUNS, { limit: 10 }, { revalidateOnFocus: false });

  const queries = queriesData?.discoveryQueries || [];
  const globalRules = globalRulesData?.discoveryFilterRules || [];
  const runs = runsData?.discoveryRuns || [];

  const handleRunDiscovery = async () => {
    setIsRunning(true);
    setRunMessage(null);
    try {
      const result = await graphqlMutateClient<{
        runDiscoverySearch: {
          queriesRun: number;
          totalResults: number;
          websitesCreated: number;
          websitesFiltered: number;
          runId: string;
        };
      }>(RUN_DISCOVERY_SEARCH);

      const r = result.runDiscoverySearch;
      setRunMessage(
        `Done! ${r.queriesRun} queries, ${r.totalResults} results, ${r.websitesCreated} websites created, ${r.websitesFiltered} filtered`
      );
      invalidateAllMatchingQuery(GET_DISCOVERY_RUNS);
    } catch (e: unknown) {
      setRunMessage(`Error: ${e instanceof Error ? e.message : "Unknown error"}`);
    } finally {
      setIsRunning(false);
    }
  };

  if (queriesLoading && queries.length === 0) {
    return <AdminLoader label="Loading discovery..." />;
  }

  return (
    <div className="max-w-7xl mx-auto p-8">
      <div className="flex items-center justify-between mb-8">
        <h1 className="text-3xl font-bold">Discovery</h1>
        <button
          onClick={handleRunDiscovery}
          disabled={isRunning}
          className="px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
              ? `Queries (${queries.length})`
              : tab === "filters"
                ? `Global Filters (${globalRules.length})`
                : `Runs (${runs.length})`}
          </button>
        ))}
      </div>

      {activeTab === "queries" && <QueriesTab queries={queries} />}
      {activeTab === "filters" && <FiltersTab rules={globalRules} queryId={null} />}
      {activeTab === "runs" && <RunsTab runs={runs} />}
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
      await graphqlMutateClient(CREATE_DISCOVERY_QUERY, {
        queryText: newQueryText.trim(),
        category: newCategory.trim() || null,
      });
      setNewQueryText("");
      setNewCategory("");
      invalidateAllMatchingQuery(GET_DISCOVERY_QUERIES);
    } catch (e: unknown) {
      alert(`Failed to create query: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleUpdate = async (id: string) => {
    try {
      await graphqlMutateClient(UPDATE_DISCOVERY_QUERY, {
        id,
        queryText: editText.trim(),
        category: editCategory.trim() || null,
      });
      setEditingId(null);
      invalidateAllMatchingQuery(GET_DISCOVERY_QUERIES);
    } catch (e: unknown) {
      alert(`Failed to update: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleToggle = async (id: string, isActive: boolean) => {
    try {
      await graphqlMutateClient(TOGGLE_DISCOVERY_QUERY, { id, isActive: !isActive });
      invalidateAllMatchingQuery(GET_DISCOVERY_QUERIES);
    } catch (e: unknown) {
      alert(`Failed to toggle: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this query? Results from previous runs are preserved.")) return;
    try {
      await graphqlMutateClient(DELETE_DISCOVERY_QUERY, { id });
      invalidateAllMatchingQuery(GET_DISCOVERY_QUERIES);
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
              <th className="px-4 py-3 text-left text-xs font-medium text-stone-500 uppercase">
                Query
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-stone-500 uppercase">
                Category
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-stone-500 uppercase">
                Status
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-stone-500 uppercase">
                Actions
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stone-200">
            {queries.map((query) => (
              <tr key={query.id} className={!query.isActive ? "opacity-50" : ""}>
                <td className="px-4 py-3">
                  {editingId === query.id ? (
                    <input
                      type="text"
                      value={editText}
                      onChange={(e) => setEditText(e.target.value)}
                      className="w-full px-2 py-1 border border-stone-300 rounded text-sm"
                      onKeyDown={(e) => e.key === "Enter" && handleUpdate(query.id)}
                    />
                  ) : (
                    <span className="text-sm text-stone-900 font-mono">{query.queryText}</span>
                  )}
                </td>
                <td className="px-4 py-3">
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
                <td className="px-4 py-3">
                  <button
                    onClick={() => handleToggle(query.id, query.isActive)}
                    className={`text-xs px-2 py-1 rounded-full ${
                      query.isActive
                        ? "bg-green-100 text-green-700"
                        : "bg-stone-100 text-stone-500"
                    }`}
                  >
                    {query.isActive ? "active" : "inactive"}
                  </button>
                </td>
                <td className="px-4 py-3 text-right">
                  {editingId === query.id ? (
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => handleUpdate(query.id)}
                        className="text-xs text-green-600 hover:text-green-800"
                      >
                        Save
                      </button>
                      <button
                        onClick={() => setEditingId(null)}
                        className="text-xs text-stone-500 hover:text-stone-700"
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => {
                          setEditingId(query.id);
                          setEditText(query.queryText);
                          setEditCategory(query.category || "");
                        }}
                        className="text-xs text-stone-500 hover:text-stone-700"
                      >
                        Edit
                      </button>
                      <button
                        onClick={() => handleDelete(query.id)}
                        className="text-xs text-red-500 hover:text-red-700"
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
  rules: FilterRule[];
  queryId: string | null;
}) {
  const [newRuleText, setNewRuleText] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editText, setEditText] = useState("");

  const handleCreate = async () => {
    if (!newRuleText.trim()) return;
    try {
      await graphqlMutateClient(CREATE_DISCOVERY_FILTER_RULE, {
        queryId,
        ruleText: newRuleText.trim(),
      });
      setNewRuleText("");
      invalidateAllMatchingQuery(GET_DISCOVERY_FILTER_RULES);
    } catch (e: unknown) {
      alert(`Failed to create rule: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleUpdate = async (id: string) => {
    try {
      await graphqlMutateClient(UPDATE_DISCOVERY_FILTER_RULE, {
        id,
        ruleText: editText.trim(),
      });
      setEditingId(null);
      invalidateAllMatchingQuery(GET_DISCOVERY_FILTER_RULES);
    } catch (e: unknown) {
      alert(`Failed to update: ${e instanceof Error ? e.message : "Unknown error"}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this filter rule?")) return;
    try {
      await graphqlMutateClient(DELETE_DISCOVERY_FILTER_RULE, { id });
      invalidateAllMatchingQuery(GET_DISCOVERY_FILTER_RULES);
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
            placeholder='e.g. "Omit government websites"'
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
                  className="text-xs text-green-600 hover:text-green-800"
                >
                  Save
                </button>
                <button
                  onClick={() => setEditingId(null)}
                  className="text-xs text-stone-500 hover:text-stone-700"
                >
                  Cancel
                </button>
              </>
            ) : (
              <>
                <span className="flex-1 text-sm text-stone-800">{rule.ruleText}</span>
                <button
                  onClick={() => {
                    setEditingId(rule.id);
                    setEditText(rule.ruleText);
                  }}
                  className="text-xs text-stone-500 hover:text-stone-700"
                >
                  Edit
                </button>
                <button
                  onClick={() => handleDelete(rule.id)}
                  className="text-xs text-red-500 hover:text-red-700"
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
              <th className="px-4 py-3 text-left text-xs font-medium text-stone-500 uppercase">
                Date
              </th>
              <th className="px-4 py-3 text-left text-xs font-medium text-stone-500 uppercase">
                Trigger
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-stone-500 uppercase">
                Queries
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-stone-500 uppercase">
                Results
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-stone-500 uppercase">
                Created
              </th>
              <th className="px-4 py-3 text-right text-xs font-medium text-stone-500 uppercase">
                Filtered
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stone-200">
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
                <td className="px-4 py-3 text-sm text-stone-900">
                  {new Date(run.startedAt).toLocaleString()}
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`text-xs px-2 py-1 rounded-full ${
                      run.triggerType === "manual"
                        ? "bg-blue-100 text-blue-700"
                        : "bg-stone-100 text-stone-600"
                    }`}
                  >
                    {run.triggerType}
                  </span>
                </td>
                <td className="px-4 py-3 text-sm text-stone-600 text-right">
                  {run.queriesExecuted}
                </td>
                <td className="px-4 py-3 text-sm text-stone-600 text-right">
                  {run.totalResults}
                </td>
                <td className="px-4 py-3 text-sm text-green-600 font-medium text-right">
                  {run.websitesCreated}
                </td>
                <td className="px-4 py-3 text-sm text-red-600 font-medium text-right">
                  {run.websitesFiltered}
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
  const { data, isLoading } = useGraphQL<{
    discoveryRunResults: RunResult[];
  }>(GET_DISCOVERY_RUN_RESULTS, { runId }, { revalidateOnFocus: false });

  const results = data?.discoveryRunResults || [];

  if (isLoading) {
    return (
      <div className="text-center py-4 text-stone-500 text-sm">Loading results...</div>
    );
  }

  const passed = results.filter((r) => r.filterResult === "passed");
  const filtered = results.filter((r) => r.filterResult === "filtered");

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
              result.filterResult === "passed"
                ? "border-green-200 bg-green-50"
                : result.filterResult === "filtered"
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
                  result.filterResult === "passed"
                    ? "bg-green-200 text-green-800"
                    : "bg-red-200 text-red-800"
                }`}
              >
                {result.filterResult}
              </span>
            </div>
            {result.filterReason && (
              <div className="text-xs text-stone-500 mt-1">{result.filterReason}</div>
            )}
            {result.relevanceScore != null && (
              <div className="text-xs text-stone-400 mt-1">
                Score: {result.relevanceScore.toFixed(2)}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
