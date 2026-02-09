"use client";

import { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import {
  useRestate,
  callService,
  invalidateService,
} from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type {
  AgentDetailResponse,
  AgentRunListResponse,
  SearchQueryResponse,
  FilterRuleResponse,
  TagKindListResult,
} from "@/lib/restate/types";

type Tab = "overview" | "queries" | "filters" | "runs" | "websites" | "posts";

export default function AgentDetailPage() {
  const params = useParams();
  const router = useRouter();
  const agentId = params.id as string;
  const [activeTab, setActiveTab] = useState<Tab>("overview");
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);

  const { data, isLoading, mutate } = useRestate<AgentDetailResponse>(
    "Agents",
    "get_agent",
    { agent_id: agentId },
    { revalidateOnFocus: false }
  );

  if (isLoading || !data) {
    return <AdminLoader label="Loading agent..." />;
  }

  const { agent } = data;
  const isCurator = agent.role === "curator";

  const tabs: { key: Tab; label: string }[] = isCurator
    ? [
        { key: "overview", label: "Overview" },
        { key: "queries", label: "Search Queries" },
        { key: "filters", label: "Filter Rules" },
        { key: "runs", label: "Runs" },
        { key: "websites", label: "Websites" },
        { key: "posts", label: "Posts" },
      ]
    : [{ key: "overview", label: "Configuration" }];

  const handleDelete = async () => {
    if (!confirm(`Delete "${agent.display_name}"? This cannot be undone.`)) return;
    setActionInProgress("delete");
    try {
      await callService("Agents", "delete_agent", { agent_id: agentId });
      invalidateService("Agents");
      router.push("/admin/agents");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleStatusChange = async (newStatus: string) => {
    setActionInProgress("status");
    try {
      await callService("Agents", "set_agent_status", {
        agent_id: agentId,
        status: newStatus,
      });
      invalidateService("Agents");
      mutate();
    } finally {
      setActionInProgress(null);
    }
  };

  const handleRunStep = async (step: string) => {
    setActionInProgress(step);
    try {
      await callService("Agents", "run_agent_step", {
        agent_id: agentId,
        step,
      });
      invalidateService("Agents");
      mutate();
    } finally {
      setActionInProgress(null);
    }
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "active":
        return "bg-green-100 text-green-800";
      case "draft":
        return "bg-yellow-100 text-yellow-800";
      case "paused":
        return "bg-gray-100 text-gray-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Back link */}
        <Link
          href="/admin/agents"
          className="text-sm text-stone-500 hover:text-stone-700 mb-4 inline-block"
        >
          &larr; Back to Agents
        </Link>

        {/* Header */}
        <div className="bg-white rounded-lg shadow px-6 py-4 mb-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold text-stone-900">
                {agent.display_name}
              </h1>
              <span
                className={`px-2 py-1 text-xs rounded-full ${
                  agent.role === "curator"
                    ? "bg-purple-100 text-purple-800"
                    : "bg-blue-100 text-blue-800"
                }`}
              >
                {agent.role}
              </span>
              <span
                className={`px-2 py-1 text-xs rounded-full ${getStatusBadge(agent.status)}`}
              >
                {agent.status}
              </span>
            </div>
            <div className="flex items-center gap-2">
              {agent.status === "draft" && (
                <button
                  onClick={() => handleStatusChange("active")}
                  disabled={!!actionInProgress}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
                >
                  {actionInProgress === "status" ? "..." : "Activate"}
                </button>
              )}
              {agent.status === "active" && (
                <button
                  onClick={() => handleStatusChange("paused")}
                  disabled={!!actionInProgress}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-200 text-stone-700 hover:bg-stone-300 disabled:opacity-50 transition-colors"
                >
                  {actionInProgress === "status" ? "..." : "Pause"}
                </button>
              )}
              {agent.status === "paused" && (
                <button
                  onClick={() => handleStatusChange("active")}
                  disabled={!!actionInProgress}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
                >
                  {actionInProgress === "status" ? "..." : "Resume"}
                </button>
              )}
              <button
                onClick={handleDelete}
                disabled={!!actionInProgress}
                className="px-3 py-1.5 rounded-lg text-sm font-medium bg-red-100 text-red-700 hover:bg-red-200 disabled:opacity-50 transition-colors"
              >
                {actionInProgress === "delete" ? "..." : "Delete"}
              </button>
            </div>
          </div>

          {/* Pipeline Run Buttons (curators only) */}
          {isCurator && agent.status !== "paused" && (
            <div className="mt-4 pt-4 border-t border-stone-200 flex items-center gap-2">
              <span className="text-sm text-stone-500 mr-2">Run:</span>
              {["discover", "extract", "enrich", "monitor"].map((step) => (
                <button
                  key={step}
                  onClick={() => handleRunStep(step)}
                  disabled={!!actionInProgress}
                  className="px-3 py-1.5 rounded-lg text-xs font-medium bg-amber-100 text-amber-800 hover:bg-amber-200 disabled:opacity-50 transition-colors capitalize"
                >
                  {actionInProgress === step ? "Running..." : step}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Tabs */}
        {tabs.length > 1 && (
          <div className="flex gap-1 mb-6 bg-white rounded-lg shadow px-2 py-1">
            {tabs.map((tab) => (
              <button
                key={tab.key}
                onClick={() => setActiveTab(tab.key)}
                className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
                  activeTab === tab.key
                    ? "bg-amber-100 text-amber-800"
                    : "text-stone-600 hover:bg-stone-100"
                }`}
              >
                {tab.label}
                {tab.key === "queries" &&
                  data.search_queries.length > 0 &&
                  ` (${data.search_queries.length})`}
                {tab.key === "filters" &&
                  data.filter_rules.length > 0 &&
                  ` (${data.filter_rules.length})`}
                {tab.key === "websites" &&
                  data.websites.length > 0 &&
                  ` (${data.websites.length})`}
                {tab.key === "posts" &&
                  data.posts_count > 0 &&
                  ` (${data.posts_count})`}
              </button>
            ))}
          </div>
        )}

        {/* Tab Content */}
        {activeTab === "overview" && (
          <OverviewTab data={data} agentId={agentId} onUpdate={mutate} />
        )}
        {activeTab === "queries" && (
          <SearchQueriesTab
            queries={data.search_queries}
            agentId={agentId}
            onUpdate={mutate}
          />
        )}
        {activeTab === "filters" && (
          <FilterRulesTab
            rules={data.filter_rules}
            agentId={agentId}
            onUpdate={mutate}
          />
        )}
        {activeTab === "runs" && <RunsTab agentId={agentId} />}
        {activeTab === "websites" && (
          <WebsitesTab websites={data.websites} />
        )}
        {activeTab === "posts" && <PostsTab agentId={agentId} />}
      </div>
    </div>
  );
}

// =============================================================================
// Overview Tab
// =============================================================================

function OverviewTab({
  data,
  agentId,
  onUpdate,
}: {
  data: AgentDetailResponse;
  agentId: string;
  onUpdate: () => void;
}) {
  const { agent, curator_config, assistant_config, required_tag_kinds } = data;

  // Editable fields
  const [editingName, setEditingName] = useState(false);
  const [nameValue, setNameValue] = useState(agent.display_name);
  const [editingPurpose, setEditingPurpose] = useState(false);
  const [purposeValue, setPurposeValue] = useState(
    curator_config?.purpose || ""
  );
  const [editingPreamble, setEditingPreamble] = useState(false);
  const [preambleValue, setPreambleValue] = useState(
    assistant_config?.preamble || ""
  );
  const [editingSchedule, setEditingSchedule] = useState(false);
  const [scheduleDiscover, setScheduleDiscover] = useState(
    curator_config?.schedule_discover || ""
  );
  const [scheduleMonitor, setScheduleMonitor] = useState(
    curator_config?.schedule_monitor || ""
  );
  const [saving, setSaving] = useState(false);

  const saveName = async () => {
    setSaving(true);
    try {
      await callService("Agents", "update_agent", {
        agent_id: agentId,
        display_name: nameValue,
      });
      invalidateService("Agents");
      onUpdate();
      setEditingName(false);
    } finally {
      setSaving(false);
    }
  };

  const savePurpose = async () => {
    if (!curator_config) return;
    setSaving(true);
    try {
      await callService("Agents", "update_curator_config", {
        agent_id: agentId,
        purpose: purposeValue,
        audience_roles: curator_config.audience_roles,
        schedule_discover: curator_config.schedule_discover,
        schedule_monitor: curator_config.schedule_monitor,
      });
      invalidateService("Agents");
      onUpdate();
      setEditingPurpose(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-6">
      {/* Display Name */}
      <div className="bg-white rounded-lg shadow px-6 py-4">
        <label className="block text-sm font-medium text-stone-500 mb-1">
          Display Name
        </label>
        {editingName ? (
          <div className="flex items-center gap-2">
            <input
              value={nameValue}
              onChange={(e) => setNameValue(e.target.value)}
              className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
              autoFocus
            />
            <button
              onClick={saveName}
              disabled={saving}
              className="px-3 py-2 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50"
            >
              Save
            </button>
            <button
              onClick={() => {
                setEditingName(false);
                setNameValue(agent.display_name);
              }}
              className="px-3 py-2 text-stone-500 text-sm"
            >
              Cancel
            </button>
          </div>
        ) : (
          <div className="flex items-center justify-between">
            <span className="text-lg text-stone-900">{agent.display_name}</span>
            <button
              onClick={() => setEditingName(true)}
              className="text-sm text-amber-600 hover:text-amber-700"
            >
              Edit
            </button>
          </div>
        )}
      </div>

      {/* Assistant Config */}
      {assistant_config && (
        <div className="bg-white rounded-lg shadow px-6 py-4">
          <label className="block text-sm font-medium text-stone-500 mb-1">
            Config Name
          </label>
          <p className="text-stone-900 mb-4">{assistant_config.config_name}</p>
          <label className="block text-sm font-medium text-stone-500 mb-1">
            Preamble
          </label>
          {editingPreamble ? (
            <div className="space-y-2">
              <textarea
                value={preambleValue}
                onChange={(e) => setPreambleValue(e.target.value)}
                rows={10}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-amber-500"
                autoFocus
              />
              <div className="flex gap-2">
                <button
                  onClick={async () => {
                    setSaving(true);
                    try {
                      await callService("Agents", "update_assistant_config", {
                        agent_id: agentId,
                        preamble: preambleValue,
                      });
                      invalidateService("Agents");
                      onUpdate();
                      setEditingPreamble(false);
                    } finally {
                      setSaving(false);
                    }
                  }}
                  disabled={saving}
                  className="px-3 py-2 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50"
                >
                  Save
                </button>
                <button
                  onClick={() => {
                    setEditingPreamble(false);
                    setPreambleValue(assistant_config.preamble);
                  }}
                  className="px-3 py-2 text-stone-500 text-sm"
                >
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <div className="flex items-start justify-between">
              <pre className="text-sm text-stone-700 whitespace-pre-wrap bg-stone-50 rounded p-3 flex-1">
                {assistant_config.preamble || "(empty)"}
              </pre>
              <button
                onClick={() => setEditingPreamble(true)}
                className="text-sm text-amber-600 hover:text-amber-700 shrink-0 ml-4"
              >
                Edit
              </button>
            </div>
          )}
        </div>
      )}

      {/* Curator Config */}
      {curator_config && (
        <>
          {/* Purpose */}
          <div className="bg-white rounded-lg shadow px-6 py-4">
            <label className="block text-sm font-medium text-stone-500 mb-1">
              Purpose
            </label>
            {editingPurpose ? (
              <div className="space-y-2">
                <textarea
                  value={purposeValue}
                  onChange={(e) => setPurposeValue(e.target.value)}
                  rows={4}
                  className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                  autoFocus
                />
                <div className="flex gap-2">
                  <button
                    onClick={savePurpose}
                    disabled={saving}
                    className="px-3 py-2 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50"
                  >
                    Save
                  </button>
                  <button
                    onClick={() => {
                      setEditingPurpose(false);
                      setPurposeValue(curator_config.purpose);
                    }}
                    className="px-3 py-2 text-stone-500 text-sm"
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              <div className="flex items-start justify-between">
                <p className="text-stone-900 whitespace-pre-wrap">
                  {curator_config.purpose || "(not set)"}
                </p>
                <button
                  onClick={() => setEditingPurpose(true)}
                  className="text-sm text-amber-600 hover:text-amber-700 shrink-0 ml-4"
                >
                  Edit
                </button>
              </div>
            )}
          </div>

          {/* Audience & Schedule */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="bg-white rounded-lg shadow px-6 py-4">
              <label className="block text-sm font-medium text-stone-500 mb-2">
                Audience Roles
              </label>
              {curator_config.audience_roles.length > 0 ? (
                <div className="flex flex-wrap gap-2">
                  {curator_config.audience_roles.map((role) => (
                    <span
                      key={role}
                      className="px-2 py-1 bg-purple-100 text-purple-800 text-xs rounded-full"
                    >
                      {role}
                    </span>
                  ))}
                </div>
              ) : (
                <p className="text-stone-400 text-sm">(none set)</p>
              )}
            </div>

            <div className="bg-white rounded-lg shadow px-6 py-4">
              <div className="flex items-center justify-between mb-2">
                <label className="text-sm font-medium text-stone-500">
                  Schedule
                </label>
                {!editingSchedule && (
                  <button
                    onClick={() => setEditingSchedule(true)}
                    className="text-sm text-amber-600 hover:text-amber-700"
                  >
                    Edit
                  </button>
                )}
              </div>
              {editingSchedule ? (
                <div className="space-y-3">
                  <div className="flex items-center gap-3">
                    <span className="text-sm text-stone-600 w-20">Discover</span>
                    <select
                      value={scheduleDiscover}
                      onChange={(e) => setScheduleDiscover(e.target.value)}
                      className="px-3 py-1.5 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                    >
                      <option value="">Manual only</option>
                      <option value="hourly">Hourly</option>
                      <option value="daily">Daily</option>
                      <option value="weekly">Weekly</option>
                    </select>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="text-sm text-stone-600 w-20">Monitor</span>
                    <select
                      value={scheduleMonitor}
                      onChange={(e) => setScheduleMonitor(e.target.value)}
                      className="px-3 py-1.5 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                    >
                      <option value="">Manual only</option>
                      <option value="hourly">Hourly</option>
                      <option value="daily">Daily</option>
                      <option value="weekly">Weekly</option>
                    </select>
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={async () => {
                        setSaving(true);
                        try {
                          await callService("Agents", "update_curator_config", {
                            agent_id: agentId,
                            purpose: curator_config.purpose,
                            audience_roles: curator_config.audience_roles,
                            schedule_discover: scheduleDiscover || null,
                            schedule_monitor: scheduleMonitor || null,
                          });
                          invalidateService("Agents");
                          onUpdate();
                          setEditingSchedule(false);
                        } finally {
                          setSaving(false);
                        }
                      }}
                      disabled={saving}
                      className="px-3 py-1.5 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50"
                    >
                      Save
                    </button>
                    <button
                      onClick={() => {
                        setEditingSchedule(false);
                        setScheduleDiscover(curator_config.schedule_discover || "");
                        setScheduleMonitor(curator_config.schedule_monitor || "");
                      }}
                      className="px-3 py-1.5 text-stone-500 text-sm"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <div className="space-y-1 text-sm">
                  <div className="flex justify-between">
                    <span className="text-stone-600">Discover</span>
                    <span className="text-stone-900 capitalize">
                      {curator_config.schedule_discover || "Manual only"}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-stone-600">Monitor</span>
                    <span className="text-stone-900 capitalize">
                      {curator_config.schedule_monitor || "Manual only"}
                    </span>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Required Tag Kinds */}
          <div className="bg-white rounded-lg shadow px-6 py-4">
            <label className="block text-sm font-medium text-stone-500 mb-2">
              Required Tag Kinds
            </label>
            {required_tag_kinds.length > 0 ? (
              <div className="flex flex-wrap gap-2">
                {required_tag_kinds.map((tk) => (
                  <span
                    key={tk.id}
                    className="px-2 py-1 bg-amber-100 text-amber-800 text-xs rounded-full"
                  >
                    {tk.display_name}
                  </span>
                ))}
              </div>
            ) : (
              <p className="text-stone-400 text-sm">(none required)</p>
            )}
          </div>
        </>
      )}
    </div>
  );
}

// =============================================================================
// Search Queries Tab
// =============================================================================

function SearchQueriesTab({
  queries,
  agentId,
  onUpdate,
}: {
  queries: SearchQueryResponse[];
  agentId: string;
  onUpdate: () => void;
}) {
  const [showAdd, setShowAdd] = useState(false);
  const [newQuery, setNewQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newQuery.trim()) return;
    setLoading(true);
    try {
      await callService("Agents", "create_search_query", {
        agent_id: agentId,
        query_text: newQuery.trim(),
      });
      invalidateService("Agents");
      onUpdate();
      setNewQuery("");
      setShowAdd(false);
    } finally {
      setLoading(false);
    }
  };

  const handleUpdate = async (id: string) => {
    setLoading(true);
    try {
      await callService("Agents", "update_search_query", {
        id,
        query_text: editValue,
      });
      invalidateService("Agents");
      onUpdate();
      setEditingId(null);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    setLoading(true);
    try {
      await callService("Agents", "delete_search_query", { id });
      invalidateService("Agents");
      onUpdate();
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bg-white rounded-lg shadow">
      <div className="px-6 py-4 border-b border-stone-200 flex items-center justify-between">
        <h2 className="font-semibold text-stone-900">Search Queries</h2>
        <button
          onClick={() => setShowAdd(!showAdd)}
          className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors"
        >
          + Add Query
        </button>
      </div>

      {showAdd && (
        <form
          onSubmit={handleAdd}
          className="px-6 py-3 border-b border-stone-200 flex items-center gap-3"
        >
          <input
            type="text"
            value={newQuery}
            onChange={(e) => setNewQuery(e.target.value)}
            placeholder='e.g. "food shelves in Minnesota"'
            className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
            autoFocus
            disabled={loading}
          />
          <button
            type="submit"
            disabled={loading || !newQuery.trim()}
            className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50 transition-colors"
          >
            Add
          </button>
          <button
            type="button"
            onClick={() => setShowAdd(false)}
            className="text-stone-500 text-sm"
          >
            Cancel
          </button>
        </form>
      )}

      {queries.length === 0 ? (
        <div className="px-6 py-8 text-center text-stone-400">
          No search queries yet. Add one to start discovering websites.
        </div>
      ) : (
        <div className="divide-y divide-stone-200">
          {queries.map((q) => (
            <div
              key={q.id}
              className="px-6 py-3 flex items-center justify-between"
            >
              {editingId === q.id ? (
                <div className="flex-1 flex items-center gap-2">
                  <input
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    className="flex-1 px-3 py-1 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                    autoFocus
                  />
                  <button
                    onClick={() => handleUpdate(q.id)}
                    disabled={loading}
                    className="text-sm text-amber-600 hover:text-amber-700"
                  >
                    Save
                  </button>
                  <button
                    onClick={() => setEditingId(null)}
                    className="text-sm text-stone-500"
                  >
                    Cancel
                  </button>
                </div>
              ) : (
                <>
                  <div className="flex items-center gap-2">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        q.is_active ? "bg-green-500" : "bg-gray-300"
                      }`}
                    />
                    <span className="text-sm text-stone-900">
                      {q.query_text}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => {
                        setEditingId(q.id);
                        setEditValue(q.query_text);
                      }}
                      className="text-xs text-stone-400 hover:text-stone-600"
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDelete(q.id)}
                      disabled={loading}
                      className="text-xs text-red-400 hover:text-red-600"
                    >
                      Delete
                    </button>
                  </div>
                </>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Filter Rules Tab
// =============================================================================

function FilterRulesTab({
  rules,
  agentId,
  onUpdate,
}: {
  rules: FilterRuleResponse[];
  agentId: string;
  onUpdate: () => void;
}) {
  const [showAdd, setShowAdd] = useState(false);
  const [newRule, setNewRule] = useState("");
  const [loading, setLoading] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newRule.trim()) return;
    setLoading(true);
    try {
      await callService("Agents", "create_filter_rule", {
        agent_id: agentId,
        rule_text: newRule.trim(),
      });
      invalidateService("Agents");
      onUpdate();
      setNewRule("");
      setShowAdd(false);
    } finally {
      setLoading(false);
    }
  };

  const handleUpdate = async (id: string) => {
    setLoading(true);
    try {
      await callService("Agents", "update_filter_rule", {
        id,
        rule_text: editValue,
      });
      invalidateService("Agents");
      onUpdate();
      setEditingId(null);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    setLoading(true);
    try {
      await callService("Agents", "delete_filter_rule", { id });
      invalidateService("Agents");
      onUpdate();
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bg-white rounded-lg shadow">
      <div className="px-6 py-4 border-b border-stone-200 flex items-center justify-between">
        <div>
          <h2 className="font-semibold text-stone-900">Filter Rules</h2>
          <p className="text-xs text-stone-400 mt-0.5">
            Plain-text rules for AI to filter discovered websites against.
          </p>
        </div>
        <button
          onClick={() => setShowAdd(!showAdd)}
          className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors"
        >
          + Add Rule
        </button>
      </div>

      {showAdd && (
        <form
          onSubmit={handleAdd}
          className="px-6 py-3 border-b border-stone-200 flex items-center gap-3"
        >
          <input
            type="text"
            value={newRule}
            onChange={(e) => setNewRule(e.target.value)}
            placeholder='e.g. "Must be a nonprofit or government organization"'
            className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
            autoFocus
            disabled={loading}
          />
          <button
            type="submit"
            disabled={loading || !newRule.trim()}
            className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm hover:bg-amber-700 disabled:opacity-50 transition-colors"
          >
            Add
          </button>
          <button
            type="button"
            onClick={() => setShowAdd(false)}
            className="text-stone-500 text-sm"
          >
            Cancel
          </button>
        </form>
      )}

      {rules.length === 0 ? (
        <div className="px-6 py-8 text-center text-stone-400">
          No filter rules yet. Add rules to control which websites are kept.
        </div>
      ) : (
        <div className="divide-y divide-stone-200">
          {rules.map((r) => (
            <div
              key={r.id}
              className="px-6 py-3 flex items-center justify-between"
            >
              {editingId === r.id ? (
                <div className="flex-1 flex items-center gap-2">
                  <input
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    className="flex-1 px-3 py-1 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                    autoFocus
                  />
                  <button
                    onClick={() => handleUpdate(r.id)}
                    disabled={loading}
                    className="text-sm text-amber-600 hover:text-amber-700"
                  >
                    Save
                  </button>
                  <button
                    onClick={() => setEditingId(null)}
                    className="text-sm text-stone-500"
                  >
                    Cancel
                  </button>
                </div>
              ) : (
                <>
                  <div className="flex items-center gap-2">
                    <span
                      className={`w-2 h-2 rounded-full ${
                        r.is_active ? "bg-green-500" : "bg-gray-300"
                      }`}
                    />
                    <span className="text-sm text-stone-900">{r.rule_text}</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => {
                        setEditingId(r.id);
                        setEditValue(r.rule_text);
                      }}
                      className="text-xs text-stone-400 hover:text-stone-600"
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => handleDelete(r.id)}
                      disabled={loading}
                      className="text-xs text-red-400 hover:text-red-600"
                    >
                      Delete
                    </button>
                  </div>
                </>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Runs Tab
// =============================================================================

function RunsTab({ agentId }: { agentId: string }) {
  const { data, isLoading } = useRestate<AgentRunListResponse>(
    "Agents",
    "list_runs",
    { agent_id: agentId, limit: 50 },
    { revalidateOnFocus: false }
  );

  const runs = data?.runs || [];

  if (isLoading) {
    return <AdminLoader label="Loading runs..." />;
  }

  const getStepColor = (step: string) => {
    switch (step) {
      case "discover":
        return "bg-blue-100 text-blue-800";
      case "extract":
        return "bg-purple-100 text-purple-800";
      case "enrich":
        return "bg-green-100 text-green-800";
      case "monitor":
        return "bg-orange-100 text-orange-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const getRunStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "bg-green-100 text-green-800";
      case "running":
        return "bg-blue-100 text-blue-800";
      case "failed":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {runs.length === 0 ? (
        <div className="px-6 py-8 text-center text-stone-400">
          No runs yet. Use the Run buttons above to trigger a pipeline step.
        </div>
      ) : (
        <table className="min-w-full divide-y divide-stone-200">
          <thead className="bg-stone-50">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Step
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Trigger
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Status
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Stats
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Started
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Duration
              </th>
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-stone-200">
            {runs.map((run) => {
              const duration =
                run.completed_at && run.started_at
                  ? Math.round(
                      (new Date(run.completed_at).getTime() -
                        new Date(run.started_at).getTime()) /
                        1000
                    )
                  : null;
              return (
                <tr key={run.id}>
                  <td className="px-6 py-3 whitespace-nowrap">
                    <span
                      className={`px-2 py-1 text-xs rounded-full capitalize ${getStepColor(run.step)}`}
                    >
                      {run.step}
                    </span>
                  </td>
                  <td className="px-6 py-3 whitespace-nowrap text-sm text-stone-600">
                    {run.trigger_type}
                  </td>
                  <td className="px-6 py-3 whitespace-nowrap">
                    <span
                      className={`px-2 py-1 text-xs rounded-full ${getRunStatusColor(run.status)}`}
                    >
                      {run.status}
                    </span>
                  </td>
                  <td className="px-6 py-3 text-xs text-stone-600">
                    {run.stats.length > 0
                      ? run.stats
                          .map((s) => `${s.stat_key}: ${s.stat_value}`)
                          .join(", ")
                      : "-"}
                  </td>
                  <td className="px-6 py-3 whitespace-nowrap text-sm text-stone-500">
                    {new Date(run.started_at).toLocaleString()}
                  </td>
                  <td className="px-6 py-3 whitespace-nowrap text-sm text-stone-500">
                    {duration !== null ? `${duration}s` : "-"}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

// =============================================================================
// Websites Tab
// =============================================================================

function WebsitesTab({
  websites,
}: {
  websites: AgentDetailResponse["websites"];
}) {
  const router = useRouter();

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {websites.length === 0 ? (
        <div className="px-6 py-8 text-center text-stone-400">
          No websites discovered yet. Run the Discover step to find websites.
        </div>
      ) : (
        <table className="min-w-full divide-y divide-stone-200">
          <thead className="bg-stone-50">
            <tr>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Domain
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Posts
              </th>
              <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                Discovered
              </th>
            </tr>
          </thead>
          <tbody className="bg-white divide-y divide-stone-200">
            {websites.map((w) => (
              <tr
                key={w.website_id}
                onClick={() => router.push(`/admin/websites/${w.website_id}`)}
                className="hover:bg-stone-50 cursor-pointer"
              >
                <td className="px-6 py-3 whitespace-nowrap font-medium text-stone-900">
                  {w.domain || w.website_id}
                </td>
                <td className="px-6 py-3 whitespace-nowrap text-sm text-stone-600">
                  {w.post_count}
                </td>
                <td className="px-6 py-3 whitespace-nowrap text-sm text-stone-500">
                  {new Date(w.discovered_at).toLocaleDateString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

// =============================================================================
// Posts Tab
// =============================================================================

function PostsTab({ agentId }: { agentId: string }) {
  const router = useRouter();
  const { data, isLoading } = useRestate<{
    posts: { id: string; title: string; status: string; created_at: string }[];
    total_count: number;
  }>(
    "Posts",
    "list",
    { agent_id: agentId, limit: 100, offset: 0 },
    { revalidateOnFocus: false }
  );

  const posts = data?.posts || [];

  if (isLoading) {
    return <AdminLoader label="Loading posts..." />;
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case "active":
        return "bg-green-100 text-green-800";
      case "pending_approval":
        return "bg-yellow-100 text-yellow-800";
      case "rejected":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  return (
    <div className="bg-white rounded-lg shadow overflow-hidden">
      {posts.length === 0 ? (
        <div className="px-6 py-8 text-center text-stone-400">
          No posts extracted yet. Run the Extract step to create posts.
        </div>
      ) : (
        <>
          <div className="px-6 py-3 border-b border-stone-200 text-sm text-stone-500">
            {data?.total_count || 0} posts
          </div>
          <table className="min-w-full divide-y divide-stone-200">
            <thead className="bg-stone-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                  Title
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                  Created
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-stone-200">
              {posts.map((post) => (
                <tr
                  key={post.id}
                  onClick={() => router.push(`/admin/posts/${post.id}`)}
                  className="hover:bg-stone-50 cursor-pointer"
                >
                  <td className="px-6 py-3 font-medium text-stone-900 max-w-md truncate">
                    {post.title}
                  </td>
                  <td className="px-6 py-3 whitespace-nowrap">
                    <span
                      className={`px-2 py-1 text-xs rounded-full ${getStatusColor(post.status)}`}
                    >
                      {post.status.replace(/_/g, " ")}
                    </span>
                  </td>
                  <td className="px-6 py-3 whitespace-nowrap text-sm text-stone-500">
                    {new Date(post.created_at).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
