"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { AgentListResponse, AgentResponse, SuggestAgentResponse } from "@/lib/restate/types";

export default function AgentsPage() {
  return (
    <Suspense fallback={<AdminLoader label="Loading agents..." />}>
      <AgentsContent />
    </Suspense>
  );
}

function AgentsContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const roleFilter = searchParams.get("role");

  const setRoleFilter = (role: string | null) => {
    const params = new URLSearchParams(searchParams.toString());
    if (role) {
      params.set("role", role);
    } else {
      params.delete("role");
    }
    router.replace(`/admin/agents?${params.toString()}`);
  };

  // Create agent wizard: step 1 = describe, step 2 = review & create
  const [createStep, setCreateStep] = useState<0 | 1 | 2>(0); // 0=closed, 1=describe, 2=review
  const [createDescription, setCreateDescription] = useState("");
  const [createName, setCreateName] = useState("");
  const [createRole, setCreateRole] = useState("curator");
  const [createPurpose, setCreatePurpose] = useState("");
  const [createQueries, setCreateQueries] = useState<string[]>([]);
  const [createRules, setCreateRules] = useState<string[]>([]);
  const [createLoading, setCreateLoading] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  const handleSuggest = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!createDescription.trim()) return;

    setCreateLoading(true);
    setCreateError(null);
    try {
      const suggestion = await callService<SuggestAgentResponse>(
        "Agents",
        "suggest_agent",
        { description: `[Role: ${createRole}] ${createDescription.trim()}` }
      );
      setCreateName(suggestion.display_name);
      setCreatePurpose(suggestion.purpose);
      setCreateQueries(suggestion.search_queries);
      setCreateRules(suggestion.filter_rules);
      setCreateStep(2);
    } catch (err: any) {
      setCreateError(err.message || "Failed to generate suggestion");
    } finally {
      setCreateLoading(false);
    }
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!createName.trim()) return;

    setCreateLoading(true);
    setCreateError(null);
    try {
      const agent = await callService<AgentResponse>("Agents", "create_agent", {
        display_name: createName.trim(),
        role: createRole,
        purpose: createRole === "curator" ? createPurpose.trim() || undefined : undefined,
      });

      // Create search queries and filter rules for curators
      if (createRole === "curator") {
        await Promise.all([
          ...createQueries.filter(q => q.trim()).map((q, i) =>
            callService("Agents", "create_search_query", {
              agent_id: agent.id,
              query_text: q.trim(),
              sort_order: i,
            })
          ),
          ...createRules.filter(r => r.trim()).map((r, i) =>
            callService("Agents", "create_filter_rule", {
              agent_id: agent.id,
              rule_text: r.trim(),
              sort_order: i,
            })
          ),
        ]);
      }

      invalidateService("Agents");
      resetCreateForm();
      router.push(`/admin/agents/${agent.id}`);
    } catch (err: any) {
      setCreateError(err.message || "Failed to create agent");
    } finally {
      setCreateLoading(false);
    }
  };

  const resetCreateForm = () => {
    setCreateStep(0);
    setCreateDescription("");
    setCreateName("");
    setCreateRole("curator");
    setCreatePurpose("");
    setCreateQueries([]);
    setCreateRules([]);
    setCreateError(null);
  };

  const { data, isLoading, error } = useRestate<AgentListResponse>(
    "Agents",
    "list_agents",
    { role: roleFilter },
    { revalidateOnFocus: false }
  );

  const agents = data?.agents || [];

  const getRoleBadge = (role: string) => {
    switch (role) {
      case "assistant":
        return "bg-blue-100 text-blue-800";
      case "curator":
        return "bg-purple-100 text-purple-800";
      default:
        return "bg-gray-100 text-gray-800";
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

  if (isLoading && agents.length === 0) {
    return <AdminLoader label="Loading agents..." />;
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Agents</h1>
          <div className="flex gap-2 items-center">
            {["all", "curator", "assistant"].map((role) => (
              <button
                key={role}
                onClick={() => setRoleFilter(role === "all" ? null : role)}
                className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                  (role === "all" && !roleFilter) || roleFilter === role
                    ? "bg-amber-600 text-white"
                    : "bg-stone-100 text-stone-700 hover:bg-stone-200"
                }`}
              >
                {role === "all" ? "All" : role.charAt(0).toUpperCase() + role.slice(1) + "s"}
              </button>
            ))}
            <button
              onClick={() => createStep === 0 ? setCreateStep(1) : resetCreateForm()}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors ml-2"
            >
              + Create Agent
            </button>
          </div>
        </div>

        {/* Step 1: Pick role & describe what you want */}
        {createStep === 1 && (
          <form
            onSubmit={handleSuggest}
            className="bg-white rounded-lg shadow px-6 py-4 mb-6 space-y-4"
          >
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-2">
                Role
              </label>
              <div className="flex gap-2">
                {[
                  { value: "curator", label: "Curator", description: "Discovers websites, extracts and monitors posts" },
                  { value: "assistant", label: "Assistant", description: "Chat agent that helps users find resources" },
                ].map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    onClick={() => setCreateRole(option.value)}
                    disabled={createLoading}
                    className={`flex-1 px-4 py-3 rounded-lg border-2 text-left transition-colors ${
                      createRole === option.value
                        ? "border-amber-500 bg-amber-50"
                        : "border-stone-200 hover:border-stone-300"
                    }`}
                  >
                    <span className="block text-sm font-medium text-stone-900">{option.label}</span>
                    <span className="block text-xs text-stone-500 mt-0.5">{option.description}</span>
                  </button>
                ))}
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">
                What kind of {createRole} do you want to create?
              </label>
              <textarea
                value={createDescription}
                onChange={(e) => setCreateDescription(e.target.value)}
                placeholder={createRole === "curator"
                  ? 'e.g. "An agent that finds food shelves and food banks in Minnesota"'
                  : 'e.g. "A chat assistant that helps people find legal aid"'}
                rows={3}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
                autoFocus
                disabled={createLoading}
              />
            </div>
            <div className="flex items-center gap-3">
              <button
                type="submit"
                disabled={createLoading || !createDescription.trim()}
                className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {createLoading ? "Generating..." : "Next"}
              </button>
              <button
                type="button"
                onClick={resetCreateForm}
                className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
              >
                Cancel
              </button>
              {createError && (
                <span className="text-red-600 text-sm">{createError}</span>
              )}
            </div>
          </form>
        )}

        {/* Step 2: Review and create */}
        {createStep === 2 && (
          <form
            onSubmit={handleCreate}
            className="bg-white rounded-lg shadow px-6 py-4 mb-6 space-y-4"
          >
            <div className="flex gap-4">
              <div className="flex-1">
                <label className="block text-sm font-medium text-stone-700 mb-1">
                  Display Name
                </label>
                <input
                  type="text"
                  value={createName}
                  onChange={(e) => setCreateName(e.target.value)}
                  className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
                  autoFocus
                  disabled={createLoading}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-stone-700 mb-1">
                  Role
                </label>
                <select
                  value={createRole}
                  onChange={(e) => setCreateRole(e.target.value)}
                  className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
                  disabled={createLoading}
                >
                  <option value="curator">Curator</option>
                  <option value="assistant">Assistant</option>
                </select>
              </div>
            </div>
            {createRole === "curator" && (
              <>
                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-1">
                    Purpose
                  </label>
                  <textarea
                    value={createPurpose}
                    onChange={(e) => setCreatePurpose(e.target.value)}
                    rows={2}
                    className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
                    disabled={createLoading}
                  />
                </div>

                {/* Search Queries */}
                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Search Queries
                  </label>
                  <div className="space-y-2">
                    {createQueries.map((q, i) => (
                      <div key={i} className="flex items-center gap-2">
                        <input
                          type="text"
                          value={q}
                          onChange={(e) => {
                            const updated = [...createQueries];
                            updated[i] = e.target.value;
                            setCreateQueries(updated);
                          }}
                          className="flex-1 px-3 py-1.5 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
                          disabled={createLoading}
                        />
                        <button
                          type="button"
                          onClick={() =>
                            setCreateQueries(createQueries.filter((_, j) => j !== i))
                          }
                          className="text-red-400 hover:text-red-600 text-xs shrink-0"
                        >
                          Remove
                        </button>
                      </div>
                    ))}
                    <button
                      type="button"
                      onClick={() => setCreateQueries([...createQueries, ""])}
                      className="text-xs text-amber-600 hover:text-amber-700"
                    >
                      + Add query
                    </button>
                  </div>
                </div>

                {/* Filter Rules */}
                <div>
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Filter Rules
                  </label>
                  <div className="space-y-2">
                    {createRules.map((r, i) => (
                      <div key={i} className="flex items-center gap-2">
                        <input
                          type="text"
                          value={r}
                          onChange={(e) => {
                            const updated = [...createRules];
                            updated[i] = e.target.value;
                            setCreateRules(updated);
                          }}
                          className="flex-1 px-3 py-1.5 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
                          disabled={createLoading}
                        />
                        <button
                          type="button"
                          onClick={() =>
                            setCreateRules(createRules.filter((_, j) => j !== i))
                          }
                          className="text-red-400 hover:text-red-600 text-xs shrink-0"
                        >
                          Remove
                        </button>
                      </div>
                    ))}
                    <button
                      type="button"
                      onClick={() => setCreateRules([...createRules, ""])}
                      className="text-xs text-amber-600 hover:text-amber-700"
                    >
                      + Add rule
                    </button>
                  </div>
                </div>
              </>
            )}
            <div className="flex items-center gap-3">
              <button
                type="submit"
                disabled={createLoading || !createName.trim()}
                className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {createLoading ? "Creating..." : "Create"}
              </button>
              <button
                type="button"
                onClick={() => setCreateStep(1)}
                className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
              >
                Back
              </button>
              <button
                type="button"
                onClick={resetCreateForm}
                className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
              >
                Cancel
              </button>
              {createError && (
                <span className="text-red-600 text-sm">{createError}</span>
              )}
            </div>
          </form>
        )}

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            Error: {error.message}
          </div>
        )}

        {agents.length === 0 ? (
          <div className="text-stone-500 text-center py-12">
            No agents found.{" "}
            <button
              onClick={() => setCreateStep(1)}
              className="text-amber-600 hover:text-amber-700 underline"
            >
              Create one
            </button>
          </div>
        ) : (
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <table className="min-w-full divide-y divide-stone-200">
              <thead className="bg-stone-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Name
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Role
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
                {agents.map((agent) => (
                  <tr
                    key={agent.id}
                    onClick={() => router.push(`/admin/agents/${agent.id}`)}
                    className="hover:bg-stone-50 cursor-pointer"
                  >
                    <td className="px-6 py-4 whitespace-nowrap font-medium text-stone-900">
                      {agent.display_name}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span
                        className={`px-2 py-1 text-xs rounded-full ${getRoleBadge(agent.role)}`}
                      >
                        {agent.role}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span
                        className={`px-2 py-1 text-xs rounded-full ${getStatusBadge(agent.status)}`}
                      >
                        {agent.status}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-500 text-sm">
                      {new Date(agent.created_at).toLocaleDateString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
