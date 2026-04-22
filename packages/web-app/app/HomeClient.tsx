"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useMemo } from "react";
import { useQuery } from "urql";
import { PublicBroadsheetQuery, PostTemplateConfigsQuery } from "@/lib/graphql/broadsheet";
import { CountiesQuery, PublicFiltersQuery } from "@/lib/graphql/public";
import { BroadsheetRenderer, PostcardWelcome, SiteFooter } from "@/components/broadsheet";
import { buildTemplateConfigMap } from "@/lib/broadsheet/prepare";
import { Header } from "@/components/Header";
import { Footer } from "@/components/Footer";
import { PostFeed } from "@/components/PostFeed";
import { CountyPicker } from "@/components/CountyPicker";

export function HomeClient() {
  // The county currently being viewed comes from the `?county=<id>` URL
  // param. When the URL is bare, we default to the "Statewide" pseudo
  // county — its edition is composed of posts tagged `statewide` and
  // is the right fallback for visitors who haven't picked yet, out-of-
  // state readers, or anyone before IP geolocation tells us better.
  const searchParams = useSearchParams();
  const explicitCountyId = searchParams?.get("county") ?? "";

  // Counties list — also the source of the Statewide pseudo id for the
  // default case. Lightweight query; runs on every render but hits an
  // in-memory urql cache after the first fetch.
  const [{ data: countiesData }] = useQuery({ query: CountiesQuery });
  const statewideCountyId = useMemo(
    () => countiesData?.counties?.find((c) => c.isPseudo)?.id ?? "",
    [countiesData]
  );

  const countyId = explicitCountyId || statewideCountyId;

  const [{ data: broadsheetData, fetching }] = useQuery({
    query: PublicBroadsheetQuery,
    variables: { countyId },
    pause: !countyId,
  });

  const [{ data: templatesData }] = useQuery({ query: PostTemplateConfigsQuery });
  const templateConfigs = buildTemplateConfigMap(templatesData?.postTemplates);

  const edition = broadsheetData?.publicBroadsheet;

  // Still waiting on counties query to tell us which id is Statewide.
  // Avoids a flash of PickerLanding on first paint when the URL is bare.
  if (!countyId && !countiesData) {
    return (
      <div className="broadsheet-page" style={{ textAlign: "center", padding: "4rem 1rem" }}>
        <p className="mono-sm" style={{ color: "rgba(255,255,255,0.5)" }}>
          Loading…
        </p>
      </div>
    );
  }

  // Counties loaded but no pseudo county exists (shouldn't happen in
  // normal deploys, but tolerate missing migration) — show the picker-
  // forward landing so the user can choose a real county.
  if (!countyId) {
    return <PickerLanding />;
  }

  // Broadsheet loaded — render the full newspaper experience.
  if (edition) {
    return (
      <div className="broadsheet-page">
        <PostcardWelcome />
        <div className="broadsheet-county-bar">
          <CountyPicker selectedId={countyId} />
        </div>
        <BroadsheetRenderer edition={edition} templateConfigs={templateConfigs} />
        <SiteFooter />
      </div>
    );
  }

  // Loading — transient state while the picker-chosen county loads.
  if (fetching) {
    return (
      <div className="broadsheet-page" style={{ textAlign: "center", padding: "4rem 1rem" }}>
        <p className="mono-sm" style={{ color: "rgba(255,255,255,0.5)" }}>
          Loading edition...
        </p>
      </div>
    );
  }

  // County was picked but has no published edition — show a targeted
  // "no current edition" state with the picker so the user can switch.
  return <NoEditionForCounty selectedId={countyId} />;
}

/**
 * Landing state when no county is in the URL. Shows a prominent picker
 * above the normal browse-by-type + recent posts layout. User picks a
 * county and we route to `/?county=<id>`.
 */
function PickerLanding() {
  return (
    <div className="app-shell">
      <Header />
      <main>
        <section className="page-section--home">
          <div className="picker-landing">
            <h2 className="picker-landing__title">Welcome to Minnesota, Together</h2>
            <p className="picker-landing__lede">
              A weekly community broadsheet for every Minnesota county. Pick yours to see what's
              happening this week, or browse statewide posts below.
            </p>
            <div className="picker-landing__picker">
              <CountyPicker selectedId="" />
            </div>
          </div>
        </section>
        <BrowseAndFeed />
      </main>
      <Footer />
    </div>
  );
}

/**
 * Inline state shown when the chosen county has no current published
 * edition — the picker stays visible so the user can switch without
 * going back to the home page.
 */
function NoEditionForCounty({ selectedId }: { selectedId: string }) {
  return (
    <div className="app-shell">
      <Header />
      <main>
        <section className="page-section--home">
          <div className="picker-landing">
            <h2 className="picker-landing__title">No edition yet</h2>
            <p className="picker-landing__lede">
              This county doesn't have a published broadsheet for this week. Pick a different
              county, or browse statewide content below.
            </p>
            <div className="picker-landing__picker">
              <CountyPicker selectedId={selectedId} />
            </div>
          </div>
        </section>
        <BrowseAndFeed />
      </main>
      <Footer />
    </div>
  );
}

function BrowseAndFeed() {
  const [{ data: filtersData }] = useQuery({ query: PublicFiltersQuery });
  const postTypes = filtersData?.publicFilters?.postTypes ?? [];
  return (
    <>
      {postTypes.length > 0 && (
        <section className="page-section--home">
          <h2 className="browse-title">Browse by type</h2>
          <ul className="browse-list">
            {postTypes.map((pt) => (
              <li key={pt.value}>
                <Link href={`/posts?post_type=${pt.value}`} className="browse-link">
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
      <section className="page-section--home-feed">
        <PostFeed title="Recent posts" showSeeMore />
      </section>
    </>
  );
}
