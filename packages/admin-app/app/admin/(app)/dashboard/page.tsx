"use client";

import { useMemo } from "react";
import Link from "next/link";
import { useQuery } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { DashboardQuery } from "@/lib/graphql/dashboard";

// ─── Component ───────────────────────────────────────────────────────────────

export default function DashboardPage() {
  const [{ data, fetching }] = useQuery({ query: DashboardQuery });

  // Derive stats from latest editions
  const stats = useMemo(() => {
    const editions = data?.latestEditions ?? [];
    let draft = 0,
      inReview = 0,
      approved = 0,
      published = 0;
    for (const e of editions) {
      if (e.status === "draft") draft++;
      else if (e.status === "in_review") inReview++;
      else if (e.status === "approved") approved++;
      else if (e.status === "published") published++;
    }
    return {
      draft,
      inReview,
      approved,
      published,
      total: editions.length,
    };
  }, [data]);

  if (fetching) {
    return <AdminLoader label="Loading dashboard..." />;
  }

  return (
    <div className="min-h-screen bg-[#FDFCFA] p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-stone-900 mb-1">
            Dashboard
          </h1>
          <p className="text-stone-500">
            {stats.published + stats.approved} of {stats.total || 87} counties
            up to date
          </p>
        </div>

        {/* Stat cards */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
          <StatCard
            value={stats.draft}
            label="Draft"
            color="bg-yellow-500"
            href="/admin/editions?status=draft"
          />
          <StatCard
            value={stats.inReview}
            label="Reviewing"
            color="bg-amber-500"
            href="/admin/editions?status=in_review"
          />
          <StatCard
            value={stats.approved}
            label="Approved"
            color="bg-emerald-500"
            subtitle={stats.approved > 0 ? "Ready to publish" : undefined}
            href="/admin/editions?status=approved"
          />
          <StatCard
            value={stats.published}
            label="Published"
            color="bg-green-500"
            subtitle={`of ${stats.total || 87} counties`}
            href="/admin/editions"
          />
        </div>

        {/* Ingestion — empty state */}
        <div className="bg-white rounded-lg shadow-sm border border-stone-200 mb-8">
          <div className="px-6 py-4 border-b border-stone-100">
            <h2 className="text-lg font-semibold text-stone-900">
              Root Signal Ingestion
            </h2>
          </div>
          <div className="px-6 py-12 flex flex-col items-center text-center">
            <div className="w-12 h-12 rounded-full bg-stone-100 flex items-center justify-center mb-4">
              <svg className="w-6 h-6 text-stone-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5" />
              </svg>
            </div>
            <h3 className="text-base font-medium text-stone-900 mb-1">
              No signals this week
            </h3>
            <p className="text-sm text-stone-500 max-w-sm mb-5">
              When Root Signal delivers new stories and topics, they'll appear here
              for triage before edition drafts are generated.
            </p>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-stone-50 border border-stone-200 text-xs text-stone-500">
                <span className="w-1.5 h-1.5 rounded-full bg-stone-300" />
                Waiting for data
              </div>
              <span className="text-xs text-stone-400">
                Last checked: never
              </span>
            </div>
          </div>
        </div>

        {/* Quick actions */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Link
            href="/admin/workflow"
            className="bg-amber-600 hover:bg-amber-700 text-white rounded-lg shadow-sm p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">Review Board</div>
            <p className="text-amber-100 text-sm">
              Drag editions through the approval pipeline
            </p>
          </Link>
          <Link
            href="/admin/editions"
            className="bg-stone-700 hover:bg-stone-800 text-white rounded-lg shadow-sm p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">All Editions</div>
            <p className="text-stone-300 text-sm">
              Browse and filter all county editions
            </p>
          </Link>
        </div>
      </div>
    </div>
  );
}

// ─── StatCard ────────────────────────────────────────────────────────────────

function StatCard({
  value,
  label,
  color,
  subtitle,
  href,
}: {
  value: number;
  label: string;
  color: string;
  subtitle?: string;
  href: string;
}) {
  return (
    <Link
      href={href}
      className="bg-white rounded-lg shadow-sm border border-stone-200 p-5 hover:border-stone-300 hover:shadow transition-all"
    >
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium text-stone-500 uppercase tracking-wide">
          {label}
        </span>
        <div className={`w-2.5 h-2.5 rounded-full ${color}`} />
      </div>
      <div className="text-3xl font-bold text-stone-900">{value}</div>
      {subtitle && (
        <div className="text-xs text-stone-400 mt-1">{subtitle}</div>
      )}
    </Link>
  );
}
