"use client";

import { useMemo } from "react";
import Link from "next/link";
import { useQuery } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Upload } from "lucide-react";
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
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <h1 className="text-3xl font-bold text-foreground mb-1">
            Dashboard
          </h1>
          <p className="text-muted-foreground">
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
        <div className="bg-card rounded-lg border border-border mb-8">
          <div className="px-6 py-4 border-b border-border">
            <h2 className="text-lg font-semibold text-foreground">
              Root Signal Ingestion
            </h2>
          </div>
          <div className="px-6 py-12 flex flex-col items-center text-center">
            <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center mb-4">
              <Upload className="w-6 h-6 text-muted-foreground" />
            </div>
            <h3 className="text-base font-medium text-foreground mb-1">
              No signals this week
            </h3>
            <p className="text-sm text-muted-foreground max-w-sm mb-5">
              When Root Signal delivers new stories and topics, they'll appear here
              for triage before edition drafts are generated.
            </p>
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-muted border border-border text-xs text-muted-foreground">
                <span className="w-1.5 h-1.5 rounded-full bg-muted-foreground/40" />
                Waiting for data
              </div>
              <span className="text-xs text-muted-foreground">
                Last checked: never
              </span>
            </div>
          </div>
        </div>

        {/* Quick actions */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Link
            href="/admin/workflow"
            className="bg-amber-600 hover:bg-amber-700 text-white rounded-lg p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">Review Board</div>
            <p className="text-amber-100 text-sm">
              Drag editions through the approval pipeline
            </p>
          </Link>
          <Link
            href="/admin/editions"
            className="bg-primary hover:bg-primary/90 text-primary-foreground rounded-lg p-5 transition-colors"
          >
            <div className="text-lg font-semibold mb-1">All Editions</div>
            <p className="text-primary-foreground/70 text-sm">
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
      className="bg-card rounded-lg border border-border p-5 hover:border-border/80 hover:shadow-sm transition-all"
    >
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
          {label}
        </span>
        <div className={`w-2.5 h-2.5 rounded-full ${color}`} />
      </div>
      <div className="text-3xl font-bold text-foreground">{value}</div>
      {subtitle && (
        <div className="text-xs text-muted-foreground mt-1">{subtitle}</div>
      )}
    </Link>
  );
}
