/**
 * Example: Client-side Search Component
 *
 * This demonstrates how to use the GraphQL API in a client component.
 * Rename to SearchClient.tsx and import in your page to use.
 */

"use client";

import { useState, useTransition } from "react";
import { graphqlFetchClient, SEARCH_ORGANIZATIONS } from "@/lib/graphql";
import type { SearchOrganizationsResult, OrganizationMatch } from "@/lib/types";

export function SearchClient() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<OrganizationMatch[]>([]);
  const [isPending, startTransition] = useTransition();
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async (searchQuery: string) => {
    if (!searchQuery.trim()) {
      setResults([]);
      return;
    }

    setError(null);

    startTransition(async () => {
      try {
        const data = await graphqlFetchClient<SearchOrganizationsResult>(
          SEARCH_ORGANIZATIONS,
          {
            query: searchQuery,
            limit: 10,
          }
        );

        setResults(data.searchOrganizationsSemantic);
      } catch (err) {
        console.error("Search failed:", err);
        setError("Failed to search. Please try again.");
      }
    });
  };

  return (
    <div className="space-y-6">
      {/* Search Input */}
      <div className="flex gap-2">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              handleSearch(query);
            }
          }}
          placeholder="Search for services..."
          className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:outline-none"
        />
        <button
          onClick={() => handleSearch(query)}
          disabled={isPending}
          className="bg-blue-600 text-white px-6 py-2 rounded-lg font-medium hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition"
        >
          {isPending ? "Searching..." : "Search"}
        </button>
      </div>

      {/* Error Message */}
      {error && (
        <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
          {error}
        </div>
      )}

      {/* Loading State */}
      {isPending && (
        <div className="text-center py-8">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
          <p className="mt-2 text-gray-600">Searching...</p>
        </div>
      )}

      {/* Results */}
      {!isPending && results.length > 0 && (
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">
            Found {results.length} results
          </h2>
          {results.map(({ organization, similarityScore }) => (
            <div
              key={organization.id}
              className="bg-white p-4 rounded-lg shadow hover:shadow-md transition"
            >
              <div className="flex justify-between items-start">
                <h3 className="text-lg font-semibold text-gray-900">
                  {organization.name}
                </h3>
                <span className="bg-blue-100 text-blue-800 text-sm px-2 py-1 rounded">
                  {Math.round(similarityScore * 100)}% match
                </span>
              </div>

              {organization.description && (
                <p className="text-gray-600 mt-2">{organization.description}</p>
              )}

              <div className="flex gap-4 mt-3 text-sm text-gray-500">
                {organization.phone && <span>📞 {organization.phone}</span>}
                {organization.website && (
                  <a
                    href={organization.website}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:underline"
                  >
                    🌐 Visit Website
                  </a>
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* No Results */}
      {!isPending && query && results.length === 0 && !error && (
        <div className="text-center py-8 text-gray-500">
          No results found for &quot;{query}&quot;. Try a different search term.
        </div>
      )}
    </div>
  );
}
