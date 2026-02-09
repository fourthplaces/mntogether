"use client";

import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useRestate } from "@/lib/restate/client";
import type { ExtractionPageResult } from "@/lib/restate/types";

interface OptionalPageResult {
  page: ExtractionPageResult | null;
}

export default function SnapshotDetailPage() {
  const { id: websiteId } = useParams<{ id: string }>();
  const searchParams = useSearchParams();
  const url = searchParams.get("url");

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
          href={`/admin/websites/${websiteId}`}
          className="text-sm text-stone-500 hover:text-stone-700 mb-2 inline-block"
        >
          &larr; Back to website
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
        <div className="bg-white border border-stone-200 rounded-lg shadow-sm p-6">
          <div className="prose prose-stone max-w-none">
            <ReactMarkdown>{page.content || ""}</ReactMarkdown>
          </div>
        </div>
      )}
    </div>
  );
}
