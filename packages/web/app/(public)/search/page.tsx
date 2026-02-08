import { restateCall } from "@/lib/restate/server";
import type { OrganizationResult, OrganizationMatch } from "@/lib/restate/types";
import Link from "next/link";

export default async function SearchPage({
  searchParams,
}: {
  searchParams: Promise<{ q?: string }>;
}) {
  const params = await searchParams;
  const query = params.q || "";

  let results: OrganizationMatch[] = [];

  if (query) {
    try {
      const data = await restateCall<OrganizationMatch[]>(
        "Posts/search_organizations",
        {
          query,
          limit: 20,
        }
      );
      results = data || [];
    } catch (error) {
      console.error("Search error:", error);
    }
  }
  

  
  return (
    <main className="min-h-screen bg-gray-50">
      <div className="container mx-auto px-4 py-8">
        <div className="mb-8">
          <Link
            href="/"
            className="text-blue-600 hover:text-blue-800 mb-4 inline-block"
          >
            ← Back to Home
          </Link>
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Search Services
          </h1>
          <p className="text-gray-600">
            Find services that can help you
          </p>
        </div>

        <SearchForm initialQuery={query} />

        {query && (
          <div className="mt-8">
            <h2 className="text-2xl font-semibold mb-4">
              Results for &quot;{query}&quot;
            </h2>

            {results.length === 0 ? (
              <p className="text-gray-600">
                No results found. Try a different search term.
              </p>
            ) : (
              <div className="space-y-4">
                {results.map(({ organization, similarity_score }) => (
                  <OrganizationCard
                    key={organization.id}
                    organization={organization}
                    similarityScore={similarity_score}
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </main>
  );
}

function SearchForm({ initialQuery }: { initialQuery: string }) {
  return (
    <form method="get" className="mb-8">
      <div className="flex gap-2">
        <input
          type="text"
          name="q"
          defaultValue={initialQuery}
          placeholder="e.g., immigration legal help in Spanish"
          className="flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        <button
          type="submit"
          className="bg-blue-600 text-white px-8 py-3 rounded-lg font-semibold hover:bg-blue-700 transition"
        >
          Search
        </button>
      </div>
    </form>
  );
}

function OrganizationCard({
  organization,
  similarityScore,
}: {
  organization: OrganizationResult;
  similarityScore: number;
}) {
  const matchPercentage = Math.round(similarityScore * 100);

  return (
    <div className="bg-white p-6 rounded-lg shadow-md hover:shadow-lg transition">
      <div className="flex justify-between items-start mb-2">
        <h3 className="text-xl font-semibold text-gray-900">
          {organization.name}
        </h3>
        <span className="bg-blue-100 text-blue-800 text-sm px-3 py-1 rounded-full">
          {matchPercentage}% match
        </span>
      </div>

      {organization.description && (
        <p className="text-gray-600 mb-3">{organization.description}</p>
      )}

      {organization.summary && (
        <p className="text-gray-500 text-sm mb-4">{organization.summary}</p>
      )}

      <div className="flex gap-4 text-sm text-gray-600">
        {organization.phone && (
          <a
            href={`tel:${organization.phone}`}
            className="hover:text-blue-600"
          >
            📞 {organization.phone}
          </a>
        )}
        {organization.website && (
          <a
            href={organization.website}
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-blue-600"
          >
            🌐 Website
          </a>
        )}
        {organization.primary_address && (
          <span>📍 {organization.primary_address}</span>
        )}
      </div>
    </div>
  );
}
