"use client";

/**
 * Sources panel — one row per `post_sources` citation for the post.
 *
 * Lists every citation (organisation or individual) with its URL,
 * snippet (if Signal provided one), confidence, and platform context.
 * The primary citation is visually distinguished; a "Set as primary"
 * control lets editors reassign which one feeds the public
 * attribution. See
 * `docs/handoff-root-signal/ADDENDUM_01_CITATIONS_AND_SOURCE_METADATA.md`
 * §4.3.
 *
 * Fields like `snippet`, `confidence`, `contentHash`, `platformId`,
 * `platformPostTypeHint`, and `retrievedAt` are populated once
 * Worktree 3's migration adds those columns to `post_sources`. Until
 * then they render as hidden rows — structurally correct, just empty.
 */

import * as React from "react";
import { ExternalLink, Star, StarOff, Clock, Gauge } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

export type AdminPostSource = {
  id: string;
  sourceUrl?: string | null;
  kind: string;
  organizationId?: string | null;
  organizationName?: string | null;
  individualId?: string | null;
  individualDisplayName?: string | null;
  retrievedAt?: string | null;
  contentHash?: string | null;
  snippet?: string | null;
  confidence?: number | null;
  platformId?: string | null;
  platformPostTypeHint?: string | null;
  isPrimary: boolean;
  firstSeenAt?: string | null;
  lastSeenAt?: string | null;
};

function formatDate(iso?: string | null): string | null {
  if (!iso) return null;
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return null;
    return d.toLocaleString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return null;
  }
}

function hostOf(url?: string | null): string | null {
  if (!url) return null;
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return null;
  }
}

function SourceCard({
  source,
  busy,
  onSetPrimary,
}: {
  source: AdminPostSource;
  busy: boolean;
  onSetPrimary: (id: string) => void;
}) {
  const displayName =
    source.kind === "individual"
      ? source.individualDisplayName
      : source.organizationName;
  const host = hostOf(source.sourceUrl);
  const retrieved = formatDate(source.retrievedAt) ?? formatDate(source.firstSeenAt);

  return (
    <div
      className={`rounded-lg border p-3 transition-colors ${
        source.isPrimary
          ? "border-amber-500/60 bg-amber-50/50"
          : "border-border bg-card"
      }`}
    >
      <div className="flex items-start justify-between gap-2 mb-1">
        <div className="flex items-center gap-2 min-w-0">
          {source.isPrimary ? (
            <Badge variant="warning" className="gap-1">
              <Star className="size-3" /> Primary
            </Badge>
          ) : (
            <Badge variant="secondary">{source.kind}</Badge>
          )}
          <span className="text-sm font-medium text-foreground truncate">
            {displayName || host || "Unknown source"}
          </span>
        </div>
        {!source.isPrimary && (
          <Button
            type="button"
            variant="outline"
            size="xs"
            onClick={() => onSetPrimary(source.id)}
            disabled={busy}
            title="Make this the primary citation"
          >
            <StarOff className="size-3 mr-1" /> Set primary
          </Button>
        )}
      </div>

      {source.sourceUrl && (
        <a
          href={source.sourceUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs text-link hover:text-link-hover break-all inline-flex items-center gap-1"
        >
          {source.sourceUrl}
          <ExternalLink className="size-3 shrink-0" />
        </a>
      )}

      {source.snippet && (
        <blockquote className="mt-2 border-l-2 border-border pl-2 text-sm text-muted-foreground italic">
          {source.snippet}
        </blockquote>
      )}

      <dl className="mt-2 grid grid-cols-2 gap-x-4 gap-y-1 text-xs text-muted-foreground">
        {retrieved && (
          <div className="flex items-center gap-1">
            <Clock className="size-3" />
            <span>{retrieved}</span>
          </div>
        )}
        {typeof source.confidence === "number" && (
          <div className="flex items-center gap-1">
            <Gauge className="size-3" />
            <span>{source.confidence}% confidence</span>
          </div>
        )}
        {source.platformId && (
          <div className="col-span-2 font-mono truncate">
            {source.platformPostTypeHint
              ? `${source.platformPostTypeHint}: ${source.platformId}`
              : source.platformId}
          </div>
        )}
        {source.contentHash && (
          <div className="col-span-2 font-mono text-[10px] truncate" title={source.contentHash}>
            {source.contentHash}
          </div>
        )}
      </dl>
    </div>
  );
}

export function SourcesPanel({
  sources,
  onSetPrimary,
}: {
  sources: AdminPostSource[];
  onSetPrimary: (postSourceId: string) => Promise<unknown>;
}) {
  const [busyId, setBusyId] = React.useState<string | null>(null);

  // Primary first so it anchors the panel. Stable sort by
  // first_seen_at for the remaining rows.
  const ordered = React.useMemo(() => {
    const copy = [...sources];
    copy.sort((a, b) => {
      if (a.isPrimary !== b.isPrimary) return a.isPrimary ? -1 : 1;
      const at = a.firstSeenAt || "";
      const bt = b.firstSeenAt || "";
      return at.localeCompare(bt);
    });
    return copy;
  }, [sources]);

  const handleSetPrimary = React.useCallback(
    async (id: string) => {
      setBusyId(id);
      try {
        await onSetPrimary(id);
      } catch (err) {
        console.error("Failed to set primary source:", err);
      } finally {
        setBusyId(null);
      }
    },
    [onSetPrimary],
  );

  return (
    <section>
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
        Sources{sources.length > 0 ? ` (${sources.length})` : ""}
      </h3>
      {ordered.length === 0 ? (
        <p className="text-sm text-text-faint italic px-1">
          No citations recorded for this post.
        </p>
      ) : (
        <div className="space-y-2">
          {ordered.map((s) => (
            <SourceCard
              key={s.id}
              source={s}
              busy={busyId !== null}
              onSetPrimary={handleSetPrimary}
            />
          ))}
        </div>
      )}
    </section>
  );
}
