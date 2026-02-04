"use client";

import { useState, useTransition } from "react";
import { graphqlMutateClient } from "@/lib/graphql/client";
import { INGEST_SITE, TRIGGER_EXTRACTION } from "@/lib/graphql/mutations";

function ResultDisplay({ error, result }: { error: string | null; result: unknown }) {
  if (error) {
    return (
      <div className="mt-6 p-4 bg-red-50 border border-red-200 rounded-lg">
        <h3 className="font-semibold text-red-800 mb-2">Error</h3>
        <p className="text-red-700">{error}</p>
      </div>
    );
  }

  if (result) {
    return (
      <div className="mt-6 p-4 bg-green-50 border border-green-200 rounded-lg">
        <h3 className="font-semibold text-green-800 mb-2">Result</h3>
        <pre className="text-sm text-green-900 overflow-auto max-h-96">
          {JSON.stringify(result, null, 2)}
        </pre>
      </div>
    );
  }

  return null;
}

export default function ExtractionPage() {
  const [siteUrl, setSiteUrl] = useState("");
  const [query, setQuery] = useState("");
  const [maxPages, setMaxPages] = useState(10);
  const [result, setResult] = useState<unknown>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  const handleIngest = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setResult(null);

    startTransition(async () => {
      try {
        const data = await graphqlMutateClient(INGEST_SITE, {
          siteUrl,
          maxPages,
        });
        setResult(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to ingest site");
      }
    });
  };

  const handleExtract = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setResult(null);

    startTransition(async () => {
      try {
        const data = await graphqlMutateClient(TRIGGER_EXTRACTION, {
          input: {
            query,
            site: siteUrl || undefined,
          },
        });
        setResult(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to trigger extraction");
      }
    });
  };

  return (
    <div className="max-w-4xl mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">Extraction Tools</h1>

      <div className="grid gap-8 md:grid-cols-2">
        {/* Site Ingest */}
        <div className="bg-white border border-stone-200 rounded-lg p-6">
          <h2 className="text-xl font-semibold mb-4">Ingest Site</h2>
          <p className="text-sm text-stone-600 mb-4">
            Crawl and process a website to extract structured data.
          </p>
          <form onSubmit={handleIngest} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">Site URL</label>
              <input
                type="url"
                value={siteUrl}
                onChange={(e) => setSiteUrl(e.target.value)}
                placeholder="https://example.com"
                className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">Max Pages</label>
              <input
                type="number"
                value={maxPages}
                onChange={(e) => setMaxPages(Number(e.target.value))}
                min={1}
                max={100}
                className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
              />
            </div>
            <button
              type="submit"
              disabled={isPending}
              className="w-full px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700 disabled:opacity-50"
            >
              {isPending ? "Processing..." : "Start Ingest"}
            </button>
          </form>
        </div>

        {/* Query Extraction */}
        <div className="bg-white border border-stone-200 rounded-lg p-6">
          <h2 className="text-xl font-semibold mb-4">Query Extraction</h2>
          <p className="text-sm text-stone-600 mb-4">
            Run an extraction query against a site or the web.
          </p>
          <form onSubmit={handleExtract} className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">Query</label>
              <textarea
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Find volunteer opportunities in Minneapolis"
                rows={3}
                className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
                required
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">
                Site URL (optional)
              </label>
              <input
                type="url"
                value={siteUrl}
                onChange={(e) => setSiteUrl(e.target.value)}
                placeholder="https://example.com"
                className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
              />
            </div>
            <button
              type="submit"
              disabled={isPending}
              className="w-full px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
            >
              {isPending ? "Processing..." : "Run Extraction"}
            </button>
          </form>
        </div>
      </div>

      {/* Results */}
      <ResultDisplay error={error} result={result} />
    </div>
  );
}
