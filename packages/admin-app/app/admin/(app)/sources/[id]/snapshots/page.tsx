"use client";

import Link from "next/link";
import { useState } from "react";
import { useParams, useSearchParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useRestate } from "@/lib/restate/client";
import type { ExtractionPageResult } from "@/lib/restate/types";

interface OptionalPageResult {
  page: ExtractionPageResult | null;
}

export default function SourceSnapshotDetailPage() {
  const { id: sourceId } = useParams<{ id: string }>();
  const searchParams = useSearchParams();
  const url = searchParams.get("url");

  const [showRaw, setShowRaw] = useState(false);

  const { data, isLoading, error } = useRestate<OptionalPageResult>(
    "Extraction",
    "get_page",
    { url: url || "" },
    { revalidateOnFocus: false }
  );

  const page = data?.page;

  if (!url) {
    return (
      <div className="max-w-5xl mx-auto">
        <div className="text-center py-12 text-stone-500">No snapshot URL specified</div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto">
      {/* Header */}
      <div className="mb-6">
        <Link
          href={`/admin/sources/${sourceId}`}
          className="text-sm text-stone-500 hover:text-stone-700 mb-2 inline-block"
        >
          &larr; Back to source
        </Link>
        <h1 className="text-2xl font-bold text-stone-900">Snapshot</h1>
        <a
          href={url}
          target="_blank"
          rel="noopener noreferrer"
          className="text-sm text-blue-600 hover:underline break-all"
        >
          {url}
        </a>
      </div>

      {isLoading && <AdminLoader />}
      {error && <div className="text-red-600 text-sm">Failed to load snapshot</div>}

      {!isLoading && !page && (
        <div className="text-center py-12 text-stone-500">Snapshot not found</div>
      )}

      {page && (
        <div className="bg-white border border-stone-200 rounded-lg shadow-sm">
          <div className="flex items-center justify-end px-6 pt-4">
            <button
              onClick={() => setShowRaw(!showRaw)}
              className="px-3 py-1.5 text-xs font-medium rounded-lg bg-stone-100 text-stone-700 hover:bg-stone-200 transition-colors"
            >
              {showRaw ? "Rendered" : "Raw"}
            </button>
          </div>
          <div className="p-6 pt-3">
            {showRaw ? (
              <pre className="whitespace-pre-wrap text-sm text-stone-700 font-mono bg-stone-50 rounded-lg p-4 overflow-x-auto">
                {page.content || ""}
              </pre>
            ) : (
              <div className="prose prose-stone max-w-none">
                <ReactMarkdown>{page.content || ""}</ReactMarkdown>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
