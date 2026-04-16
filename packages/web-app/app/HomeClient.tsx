"use client";

import Link from "next/link";
import { useQuery } from "urql";
import { PublicBroadsheetQuery, PostTemplateConfigsQuery } from "@/lib/graphql/broadsheet";
import { PublicFiltersQuery } from "@/lib/graphql/public";
import { BroadsheetRenderer, PostcardWelcome, SiteFooter } from "@/components/broadsheet";
import { buildTemplateConfigMap } from "@/lib/broadsheet/prepare";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";
import { PostFeed } from "@/components/PostFeed";

// Default county for MVP — Yellow Medicine County (has published test edition)
// TODO: derive from user location or URL param
const DEFAULT_COUNTY_ID = "323160e5-0b6c-4d29-bc36-869411639857";

export function HomeClient() {
  const [{ data: broadsheetData, fetching, error }] = useQuery({
    query: PublicBroadsheetQuery,
    variables: { countyId: DEFAULT_COUNTY_ID },
  });

  const [{ data: templatesData }] = useQuery({ query: PostTemplateConfigsQuery });
  const templateConfigs = buildTemplateConfigMap(templatesData?.postTemplates);

  const edition = broadsheetData?.publicBroadsheet;

  // Broadsheet loaded — render the full newspaper experience
  // PostcardWelcome sits in the green above the paper, SiteFooter below
  if (edition) {
    return (
      <div className="broadsheet-page">
        <PostcardWelcome />
        <BroadsheetRenderer edition={edition} templateConfigs={templateConfigs} />
        <SiteFooter />
      </div>
    );
  }

  // Still loading — show on green background
  if (fetching) {
    return (
      <div className="broadsheet-page" style={{ textAlign: 'center', padding: '4rem 1rem' }}>
        <p className="mono-sm" style={{ color: 'rgba(255,255,255,0.5)' }}>Loading edition...</p>
      </div>
    );
  }

  // No published edition or error — fall back to card-based UI with standard chrome
  return <FallbackHome />;
}

/**
 * Fallback homepage when no published edition exists for the county.
 * Shows the original browse-by-type + recent posts layout with standard header/footer.
 */
function FallbackHome() {
  const [{ data: filtersData }] = useQuery({ query: PublicFiltersQuery });

  const postTypes = filtersData?.publicFilters?.postTypes ?? [];

  return (
    <div className="app-shell">
      <Header />
      <main>
        {/* Post type index */}
        {postTypes.length > 0 && (
          <section className="page-section--home">
            <h2 className="browse-title">Browse by type</h2>
            <ul className="browse-list">
              {postTypes.map((pt) => (
                <li key={pt.value}>
                  <Link
                    href={`/posts?post_type=${pt.value}`}
                    className="browse-link"
                  >
                    {pt.displayName}
                  </Link>
                  {pt.description && (
                    <span className="browse-description"> &mdash; {pt.description}</span>
                  )}
                </li>
              ))}
            </ul>
          </section>
        )}

        {/* Recent posts */}
        <section className="page-section--home-feed">
          <PostFeed title="Recent posts" showSeeMore />
        </section>
      </main>
      <Footer />
    </div>
  );
}
