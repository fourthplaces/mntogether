"use client";

import { useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  EditionDetailQuery,
  GenerateEditionMutation,
  PublishEditionMutation,
  ArchiveEditionMutation,
  RemovePostFromEditionMutation,
} from "@/lib/graphql/editions";

export default function EditionDetailPage() {
  return <EditionDetailContent />;
}

function EditionDetailContent() {
  const params = useParams();
  const router = useRouter();
  const id = params.id as string;
  const [actionError, setActionError] = useState<string | null>(null);

  const [{ data, fetching, error }, refetchEdition] = useQuery({
    query: EditionDetailQuery,
    variables: { id },
  });

  const [{ fetching: generating }, generateEdition] = useMutation(GenerateEditionMutation);
  const [{ fetching: publishing }, publishEdition] = useMutation(PublishEditionMutation);
  const [{ fetching: archiving }, archiveEdition] = useMutation(ArchiveEditionMutation);
  const [, removePost] = useMutation(RemovePostFromEditionMutation);

  const edition = data?.edition;

  const handleAction = async (
    action: "generate" | "publish" | "archive",
  ) => {
    setActionError(null);
    try {
      const fns = { generate: generateEdition, publish: publishEdition, archive: archiveEdition };
      const result = await fns[action](
        { id },
        { additionalTypenames: ["Edition", "EditionConnection"] }
      );
      if (result.error) throw result.error;
      refetchEdition({ requestPolicy: "network-only" });
    } catch (err: any) {
      setActionError(err.message || `Failed to ${action} edition`);
    }
  };

  const handleRemovePost = async (slotId: string) => {
    setActionError(null);
    try {
      const result = await removePost(
        { slotId },
        { additionalTypenames: ["Edition"] }
      );
      if (result.error) throw result.error;
      refetchEdition({ requestPolicy: "network-only" });
    } catch (err: any) {
      setActionError(err.message || "Failed to remove post");
    }
  };

  // ─── Loading / error ─────────────────────────────────────────────────
  if (fetching && !edition) {
    return <AdminLoader label="Loading edition..." />;
  }

  if (error || !edition) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-5xl mx-auto">
          <button
            onClick={() => router.push("/admin/editions")}
            className="text-sm text-stone-500 hover:text-stone-700 mb-4"
          >
            &larr; Back to Editions
          </button>
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
            {error?.message || "Edition not found"}
          </div>
        </div>
      </div>
    );
  }

  const isActioning = generating || publishing || archiving;
  const sortedRows = [...edition.rows].sort((a, b) => a.sortOrder - b.sortOrder);

  // ─── Render ──────────────────────────────────────────────────────────
  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-5xl mx-auto">
        {/* Back link */}
        <button
          onClick={() => router.push("/admin/editions")}
          className="text-sm text-stone-500 hover:text-stone-700 mb-4 inline-block"
        >
          &larr; Back to Editions
        </button>

        {/* Header card */}
        <div className="bg-white rounded-lg shadow px-6 py-5 mb-6">
          <div className="flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-bold text-stone-900">
                {edition.title || `${edition.county.name} Edition`}
              </h1>
              <div className="flex items-center gap-3 mt-2 text-sm text-stone-500">
                <span className="font-medium text-stone-700">{edition.county.name} County</span>
                <span>&middot;</span>
                <span>
                  {formatDateLong(edition.periodStart)} — {formatDateLong(edition.periodEnd)}
                </span>
                <span>&middot;</span>
                <StatusBadge status={edition.status} />
              </div>
              {edition.publishedAt && (
                <div className="text-xs text-stone-400 mt-1">
                  Published {new Date(edition.publishedAt).toLocaleString()}
                </div>
              )}
            </div>

            {/* Action buttons */}
            <div className="flex gap-2 shrink-0">
              {edition.status === "draft" && (
                <>
                  <button
                    onClick={() => handleAction("generate")}
                    disabled={isActioning}
                    className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-200 text-stone-700 hover:bg-stone-300 disabled:opacity-50 transition-colors"
                  >
                    {generating ? "Regenerating..." : "Regenerate"}
                  </button>
                  <button
                    onClick={() => handleAction("publish")}
                    disabled={isActioning}
                    className="px-3 py-1.5 rounded-lg text-sm font-medium bg-green-600 text-white hover:bg-green-700 disabled:opacity-50 transition-colors"
                  >
                    {publishing ? "Publishing..." : "Publish"}
                  </button>
                </>
              )}
              {edition.status === "published" && (
                <button
                  onClick={() => handleAction("archive")}
                  disabled={isActioning}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-200 text-stone-700 hover:bg-stone-300 disabled:opacity-50 transition-colors"
                >
                  {archiving ? "Archiving..." : "Archive"}
                </button>
              )}
            </div>
          </div>

          {actionError && (
            <div className="mt-3 text-sm text-red-600 bg-red-50 px-3 py-2 rounded">
              {actionError}
            </div>
          )}
        </div>

        {/* Summary stats */}
        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="bg-white rounded-lg shadow px-4 py-3 text-center">
            <div className="text-2xl font-bold text-stone-900">{sortedRows.length}</div>
            <div className="text-xs text-stone-500 uppercase tracking-wider">Rows</div>
          </div>
          <div className="bg-white rounded-lg shadow px-4 py-3 text-center">
            <div className="text-2xl font-bold text-stone-900">
              {sortedRows.reduce((sum, r) => sum + r.slots.length, 0)}
            </div>
            <div className="text-xs text-stone-500 uppercase tracking-wider">Posts Placed</div>
          </div>
          <div className="bg-white rounded-lg shadow px-4 py-3 text-center">
            <div className="text-2xl font-bold text-stone-900">
              {new Set(sortedRows.map((r) => r.rowTemplate.slug)).size}
            </div>
            <div className="text-xs text-stone-500 uppercase tracking-wider">Unique Templates</div>
          </div>
        </div>

        {/* Broadsheet rows */}
        {sortedRows.length === 0 ? (
          <div className="text-stone-500 text-center py-12 bg-white rounded-lg shadow">
            <div className="text-4xl mb-2">📄</div>
            <p>No rows yet. Click &ldquo;Regenerate&rdquo; to populate the layout.</p>
          </div>
        ) : (
          <div className="space-y-4">
            {sortedRows.map((row, rowIdx) => (
              <div key={row.id} className="bg-white rounded-lg shadow overflow-hidden">
                {/* Row header */}
                <div className="px-5 py-3 bg-stone-50 border-b border-stone-200 flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <span className="text-xs font-mono text-stone-400 bg-stone-200 rounded px-1.5 py-0.5">
                      Row {rowIdx + 1}
                    </span>
                    <span className="text-sm font-medium text-stone-700">
                      {row.rowTemplate.displayName}
                    </span>
                    <span className="text-xs text-stone-400">
                      {row.rowTemplate.slug}
                    </span>
                  </div>
                  <span className="text-xs text-stone-400">
                    {row.slots.length} post{row.slots.length !== 1 ? "s" : ""}
                  </span>
                </div>

                {/* Slots */}
                {row.slots.length === 0 ? (
                  <div className="px-5 py-4 text-sm text-stone-400 italic">
                    Empty row — no posts placed
                  </div>
                ) : (
                  <div className="divide-y divide-stone-100">
                    {[...row.slots]
                      .sort((a, b) => a.slotIndex - b.slotIndex)
                      .map((slot) => (
                        <div
                          key={slot.id}
                          className="px-5 py-3 flex items-center justify-between hover:bg-stone-50"
                        >
                          <div className="flex items-center gap-3 min-w-0">
                            <span className="shrink-0 w-6 h-6 rounded bg-stone-100 text-stone-500 text-xs font-mono flex items-center justify-center">
                              {slot.slotIndex}
                            </span>
                            <div className="min-w-0">
                              <div className="text-sm font-medium text-stone-900 truncate">
                                {slot.post.title}
                              </div>
                              <div className="flex items-center gap-2 mt-0.5">
                                <PostTypeBadge type={slot.post.postType} />
                                <WeightBadge weight={slot.post.weight} />
                                <span className="text-xs text-stone-400">
                                  template: {slot.postTemplate}
                                </span>
                              </div>
                            </div>
                          </div>
                          <div className="flex items-center gap-2 shrink-0">
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                router.push(`/admin/posts/${slot.post.id}`);
                              }}
                              className="text-xs text-amber-600 hover:text-amber-700 font-medium"
                            >
                              View Post
                            </button>
                            {edition.status === "draft" && (
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleRemovePost(slot.id);
                                }}
                                className="text-xs text-red-500 hover:text-red-700 font-medium"
                              >
                                Remove
                              </button>
                            )}
                          </div>
                        </div>
                      ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Helper components ────────────────────────────────────────────────────────

function StatusBadge({ status }: { status: string }) {
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
}

function PostTypeBadge({ type }: { type: string | null | undefined }) {
  if (!type) return null;
  const colors: Record<string, string> = {
    story: "bg-blue-100 text-blue-700",
    notice: "bg-amber-100 text-amber-700",
    exchange: "bg-purple-100 text-purple-700",
    event: "bg-pink-100 text-pink-700",
    spotlight: "bg-green-100 text-green-700",
    reference: "bg-stone-100 text-stone-600",
  };
  return (
    <span className={`px-1.5 py-0.5 text-[10px] rounded font-medium ${colors[type] || "bg-stone-100 text-stone-600"}`}>
      {type}
    </span>
  );
}

function WeightBadge({ weight }: { weight: string | null | undefined }) {
  if (!weight) return null;
  const colors: Record<string, string> = {
    heavy: "bg-stone-800 text-white",
    medium: "bg-stone-400 text-white",
    light: "bg-stone-200 text-stone-600",
  };
  return (
    <span className={`px-1.5 py-0.5 text-[10px] rounded font-medium ${colors[weight] || "bg-stone-100 text-stone-600"}`}>
      {weight}
    </span>
  );
}

function formatDateLong(dateStr: string): string {
  const d = new Date(dateStr + "T00:00:00");
  return d.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
}
