"use client";

import { useState, useMemo } from "react";
import { useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  EditionsListQuery,
  CountiesQuery,
  CreateEditionMutation,
  GenerateEditionMutation,
  BatchGenerateEditionsMutation,
} from "@/lib/graphql/editions";

export default function EditionsPage() {
  return <EditionsContent />;
}

function EditionsContent() {
  const router = useRouter();
  const [countyFilter, setCountyFilter] = useState<string>("");
  const [statusFilter, setStatusFilter] = useState<string>("");
  const [showCreate, setShowCreate] = useState(false);
  const [showBatch, setShowBatch] = useState(false);

  // ─── Queries ────────────────────────────────────────────────────────
  const [{ data: countiesData }] = useQuery({ query: CountiesQuery });
  const [{ data, fetching, error }] = useQuery({
    query: EditionsListQuery,
    variables: {
      countyId: countyFilter || null,
      status: statusFilter || null,
      limit: 50,
      offset: 0,
    },
  });

  const counties = useMemo(() => {
    const list = countiesData?.counties || [];
    return [...list].sort((a, b) => a.name.localeCompare(b.name));
  }, [countiesData]);

  const editions = data?.editions?.editions || [];
  const totalCount = data?.editions?.totalCount ?? 0;

  // ─── Create single edition ──────────────────────────────────────────
  const [createCounty, setCreateCounty] = useState("");
  const [createStart, setCreateStart] = useState("");
  const [createEnd, setCreateEnd] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [{ fetching: creating }, createEdition] = useMutation(CreateEditionMutation);
  const [, generateEdition] = useMutation(GenerateEditionMutation);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!createCounty || !createStart || !createEnd) return;
    setCreateError(null);
    try {
      const result = await createEdition(
        { countyId: createCounty, periodStart: createStart, periodEnd: createEnd },
        { additionalTypenames: ["Edition", "EditionConnection"] }
      );
      if (result.error) throw result.error;
      const id = result.data?.createEdition?.id;
      if (id) {
        // Auto-generate layout
        const genResult = await generateEdition({ id }, { additionalTypenames: ["Edition"] });
        if (genResult.error) {
          setCreateError(`Edition created but layout generation failed: ${genResult.error.message}`);
          return;
        }
        router.push(`/admin/editions/${id}`);
      }
    } catch (err: any) {
      setCreateError(err.message || "Failed to create edition");
    }
  };

  // ─── Batch generate ─────────────────────────────────────────────────
  const [batchStart, setBatchStart] = useState("");
  const [batchEnd, setBatchEnd] = useState("");
  const [batchResult, setBatchResult] = useState<{ created: number; failed: number; totalCounties: number } | null>(null);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [{ fetching: batching }, batchGenerate] = useMutation(BatchGenerateEditionsMutation);

  const handleBatch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!batchStart || !batchEnd) return;
    setBatchError(null);
    setBatchResult(null);
    try {
      const result = await batchGenerate(
        { periodStart: batchStart, periodEnd: batchEnd },
        { additionalTypenames: ["Edition", "EditionConnection"] }
      );
      if (result.error) throw result.error;
      if (result.data?.batchGenerateEditions) {
        setBatchResult(result.data.batchGenerateEditions);
      }
    } catch (err: any) {
      setBatchError(err.message || "Batch generation failed");
    }
  };

  // ─── Status badge ───────────────────────────────────────────────────
  const statusBadge = (status: string) => {
    const styles: Record<string, string> = {
      draft: "bg-yellow-100 text-yellow-800",
      published: "bg-green-100 text-green-800",
      archived: "bg-stone-100 text-stone-600",
    };
    return (
      <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${styles[status] || "bg-stone-100 text-stone-600"}`}>
        {status}
      </span>
    );
  };

  // ─── Render ─────────────────────────────────────────────────────────
  if (fetching && editions.length === 0 && !data) {
    return <AdminLoader label="Loading editions..." />;
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div>
            <h1 className="text-3xl font-bold text-stone-900">Editions</h1>
            <p className="text-stone-500 text-sm mt-1">
              {totalCount} edition{totalCount !== 1 ? "s" : ""} found
            </p>
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => { setShowBatch(!showBatch); setShowCreate(false); }}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-200 text-stone-700 hover:bg-stone-300 transition-colors"
            >
              Batch Generate
            </button>
            <button
              onClick={() => { setShowCreate(!showCreate); setShowBatch(false); }}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-amber-600 text-white hover:bg-amber-700 transition-colors"
            >
              + Create Edition
            </button>
          </div>
        </div>

        {/* Batch generate form */}
        {showBatch && (
          <div className="bg-white rounded-lg shadow px-5 py-4 mb-6">
            <h2 className="text-sm font-semibold text-stone-700 mb-3">
              Batch Generate — All 87 Counties
            </h2>
            <form onSubmit={handleBatch} className="flex items-end gap-3">
              <div>
                <label className="block text-xs text-stone-500 mb-1">Period start</label>
                <input
                  type="date"
                  value={batchStart}
                  onChange={(e) => setBatchStart(e.target.value)}
                  className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                  disabled={batching}
                />
              </div>
              <div>
                <label className="block text-xs text-stone-500 mb-1">Period end</label>
                <input
                  type="date"
                  value={batchEnd}
                  onChange={(e) => setBatchEnd(e.target.value)}
                  className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                  disabled={batching}
                />
              </div>
              <button
                type="submit"
                disabled={batching || !batchStart || !batchEnd}
                className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {batching ? "Generating..." : "Generate All"}
              </button>
              <button
                type="button"
                onClick={() => { setShowBatch(false); setBatchResult(null); setBatchError(null); }}
                className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
              >
                Cancel
              </button>
            </form>
            {batchError && (
              <div className="mt-3 text-sm text-red-600">{batchError}</div>
            )}
            {batchResult && (
              <div className="mt-3 text-sm text-stone-700 bg-stone-50 rounded-lg px-4 py-3">
                Created <span className="font-semibold text-green-700">{batchResult.created}</span> editions
                {batchResult.failed > 0 && (
                  <>, <span className="font-semibold text-red-600">{batchResult.failed}</span> failed</>
                )}
                {" "}out of {batchResult.totalCounties} counties.
              </div>
            )}
          </div>
        )}

        {/* Create single edition form */}
        {showCreate && (
          <form onSubmit={handleCreate} className="bg-white rounded-lg shadow px-5 py-4 mb-6">
            <h2 className="text-sm font-semibold text-stone-700 mb-3">Create Single Edition</h2>
            <div className="flex items-end gap-3">
              <div className="flex-1">
                <label className="block text-xs text-stone-500 mb-1">County</label>
                <select
                  value={createCounty}
                  onChange={(e) => setCreateCounty(e.target.value)}
                  className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                  disabled={creating}
                >
                  <option value="">Select county...</option>
                  {counties.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-xs text-stone-500 mb-1">Period start</label>
                <input
                  type="date"
                  value={createStart}
                  onChange={(e) => setCreateStart(e.target.value)}
                  className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                  disabled={creating}
                />
              </div>
              <div>
                <label className="block text-xs text-stone-500 mb-1">Period end</label>
                <input
                  type="date"
                  value={createEnd}
                  onChange={(e) => setCreateEnd(e.target.value)}
                  className="px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                  disabled={creating}
                />
              </div>
              <button
                type="submit"
                disabled={creating || !createCounty || !createStart || !createEnd}
                className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {creating ? "Creating..." : "Create & Generate"}
              </button>
              <button
                type="button"
                onClick={() => { setShowCreate(false); setCreateError(null); }}
                className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
              >
                Cancel
              </button>
            </div>
            {createError && (
              <div className="mt-3 text-sm text-red-600">{createError}</div>
            )}
          </form>
        )}

        {/* Filters */}
        <div className="flex gap-3 mb-6">
          <select
            value={countyFilter}
            onChange={(e) => setCountyFilter(e.target.value)}
            className="px-3 py-2 border border-stone-300 rounded-lg text-sm bg-white focus:outline-none focus:ring-2 focus:ring-amber-500"
          >
            <option value="">All counties</option>
            {counties.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
          <div className="flex rounded-lg border border-stone-300 overflow-hidden">
            {["", "draft", "published", "archived"].map((s) => (
              <button
                key={s}
                onClick={() => setStatusFilter(s)}
                className={`px-3 py-2 text-sm font-medium transition-colors ${
                  statusFilter === s
                    ? "bg-amber-100 text-amber-800"
                    : "bg-white text-stone-600 hover:bg-stone-50"
                }`}
              >
                {s === "" ? "All" : s.charAt(0).toUpperCase() + s.slice(1)}
              </button>
            ))}
          </div>
        </div>

        {/* Error */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            Error: {error.message}
          </div>
        )}

        {/* Table */}
        {editions.length === 0 ? (
          <div className="text-stone-500 text-center py-12">
            <div className="text-4xl mb-2">📰</div>
            No editions found. Create one to get started.
          </div>
        ) : (
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <table className="min-w-full divide-y divide-stone-200">
              <thead className="bg-stone-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    County
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Period
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Rows
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-stone-500 uppercase tracking-wider">
                    Created
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-stone-200">
                {editions.map((ed) => (
                  <tr
                    key={ed.id}
                    onClick={() => router.push(`/admin/editions/${ed.id}`)}
                    className="hover:bg-stone-50 cursor-pointer"
                  >
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="font-medium text-stone-900">
                        {ed.county.name}
                      </div>
                      {ed.title && (
                        <div className="text-xs text-stone-500 truncate max-w-xs">
                          {ed.title}
                        </div>
                      )}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-700">
                      {formatDate(ed.periodStart)} — {formatDate(ed.periodEnd)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      {statusBadge(ed.status)}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-500">
                      {ed.rows.length} row{ed.rows.length !== 1 ? "s" : ""}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-500">
                      {new Date(ed.createdAt).toLocaleDateString()}
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

function formatDate(dateStr: string): string {
  const d = new Date(dateStr + "T00:00:00");
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}
