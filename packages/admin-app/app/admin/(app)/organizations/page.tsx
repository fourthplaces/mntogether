"use client";

import { Suspense, useState } from "react";
import { useRouter } from "next/navigation";
import { useRestate, callService, invalidateService } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type { OrganizationListResult } from "@/lib/restate/types";

export default function OrganizationsPage() {
  return (
    <Suspense fallback={<AdminLoader label="Loading organizations..." />}>
      <OrganizationsContent />
    </Suspense>
  );
}

function OrganizationsContent() {
  const router = useRouter();
  const [showAddForm, setShowAddForm] = useState(false);
  const [addName, setAddName] = useState("");
  const [addDescription, setAddDescription] = useState("");
  const [addLoading, setAddLoading] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);

  const { data, isLoading, error } = useRestate<OrganizationListResult>(
    "Organizations",
    "list",
    {},
    { revalidateOnFocus: false }
  );

  const organizations = data?.organizations || [];

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!addName.trim()) return;

    setAddLoading(true);
    setAddError(null);
    try {
      const result = await callService<{ id: string }>("Organizations", "create", {
        name: addName.trim(),
        description: addDescription.trim() || null,
      });
      invalidateService("Organizations");
      setAddName("");
      setAddDescription("");
      setShowAddForm(false);
      if (result?.id) {
        router.push(`/admin/organizations/${result.id}`);
      }
    } catch (err: any) {
      setAddError(err.message || "Failed to create organization");
    } finally {
      setAddLoading(false);
    }
  };

  if (isLoading && organizations.length === 0) {
    return <AdminLoader label="Loading organizations..." />;
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        <div className="flex items-center justify-between mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Organizations</h1>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors"
          >
            + Add Organization
          </button>
        </div>

        {showAddForm && (
          <form
            onSubmit={handleAdd}
            className="bg-white rounded-lg shadow px-4 py-3 mb-6 flex items-center gap-3"
          >
            <input
              type="text"
              value={addName}
              onChange={(e) => setAddName(e.target.value)}
              placeholder="Organization name"
              className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
              autoFocus
              disabled={addLoading}
            />
            <input
              type="text"
              value={addDescription}
              onChange={(e) => setAddDescription(e.target.value)}
              placeholder="Description (optional)"
              className="flex-1 px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent"
              disabled={addLoading}
            />
            <button
              type="submit"
              disabled={addLoading || !addName.trim()}
              className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {addLoading ? "Adding..." : "Add"}
            </button>
            <button
              type="button"
              onClick={() => {
                setShowAddForm(false);
                setAddName("");
                setAddDescription("");
                setAddError(null);
              }}
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

        {organizations.length === 0 ? (
          <div className="text-stone-500 text-center py-12">
            No organizations found
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
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Websites
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Social Profiles
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Created
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-stone-200">
                {organizations.map((org) => (
                  <tr
                    key={org.id}
                    onClick={() => router.push(`/admin/organizations/${org.id}`)}
                    className="hover:bg-stone-50 cursor-pointer"
                  >
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="font-medium text-stone-900">{org.name}</div>
                      {org.description && (
                        <div className="text-sm text-stone-500 truncate max-w-md">
                          {org.description}
                        </div>
                      )}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span
                        className={`px-2 py-1 text-xs rounded-full font-medium ${
                          org.status === "approved"
                            ? "bg-green-100 text-green-800"
                            : org.status === "pending_review"
                              ? "bg-yellow-100 text-yellow-800"
                              : org.status === "rejected"
                                ? "bg-red-100 text-red-800"
                                : "bg-gray-100 text-gray-800"
                        }`}
                      >
                        {org.status.replace(/_/g, " ")}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-600">
                      {org.website_count}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-600">
                      {org.social_profile_count}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-stone-500 text-sm">
                      {new Date(org.created_at).toLocaleDateString()}
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
