"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { AgentListResponse, AgentResponse } from "@/lib/restate/types";

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

  // Create agent form
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [createName, setCreateName] = useState("");
  const [createRole, setCreateRole] = useState("curator");
  const [createPurpose, setCreatePurpose] = useState("");
  const [createLoading, setCreateLoading] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

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
      invalidateService("Agents");
      setCreateName("");
      setCreatePurpose("");
      setShowCreateForm(false);
      router.push(`/admin/agents/${agent.id}`);
    } catch (err: any) {
      setCreateError(err.message || "Failed to create agent");
    } finally {
      setCreateLoading(false);
    }
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
              onClick={() => setShowCreateForm(!showCreateForm)}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors ml-2"
            >
              + Create Agent
            </button>
          </div>
        </div>

        {showCreateForm && (
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
                  placeholder="e.g. Food Shelf Curator"
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
              <div>
                <label className="block text-sm font-medium text-stone-700 mb-1">
                  Purpose
                </label>
                <textarea
                  value={createPurpose}
                  onChange={(e) => setCreatePurpose(e.target.value)}
                  placeholder="Describe what this curator should find and extract..."
                  rows={2}
                  className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
                  disabled={createLoading}
                />
              </div>
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
                onClick={() => {
                  setShowCreateForm(false);
                  setCreateName("");
                  setCreatePurpose("");
                  setCreateError(null);
                }}
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
              onClick={() => setShowCreateForm(true)}
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
