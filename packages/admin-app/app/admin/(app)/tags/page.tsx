"use client";

import { useState, useMemo } from "react";
import { useQuery } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { TagKindsQuery, TagsQuery } from "@/lib/graphql/tags";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Search,
  Lock,
  Pencil,
  Eye,
  AlertCircle,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import type { TagKind, Tag } from "@/gql/graphql";

export default function TagsPage() {
  const [{ data: kindsData, fetching: kindsLoading }] = useQuery({
    query: TagKindsQuery,
  });
  const [{ data: tagsData, fetching: tagsLoading }] = useQuery({
    query: TagsQuery,
  });
  const [search, setSearch] = useState("");
  const [expandedKinds, setExpandedKinds] = useState<Set<string>>(new Set());

  const tagsByKind = useMemo(() => {
    const map: Record<string, Tag[]> = {};
    for (const tag of tagsData?.tags || []) {
      if (!map[tag.kind]) map[tag.kind] = [];
      map[tag.kind].push(tag);
    }
    return map;
  }, [tagsData]);

  // Partition kinds into locked (fixed values) and open (expandable)
  const { lockedKinds, openKinds } = useMemo(() => {
    const all = kindsData?.tagKinds || [];
    return {
      lockedKinds: all.filter((k) => k.locked),
      openKinds: all.filter((k) => !k.locked),
    };
  }, [kindsData]);

  // Filter tags by search
  const filteredTagsByKind = useMemo(() => {
    if (!search.trim()) return tagsByKind;
    const q = search.toLowerCase();
    const filtered: Record<string, Tag[]> = {};
    for (const [kind, tags] of Object.entries(tagsByKind)) {
      const matches = tags.filter(
        (t) =>
          t.value.toLowerCase().includes(q) ||
          t.displayName?.toLowerCase().includes(q) ||
          t.description?.toLowerCase().includes(q)
      );
      if (matches.length > 0) filtered[kind] = matches;
    }
    return filtered;
  }, [tagsByKind, search]);

  // Check if a kind has matching tags (for search filtering)
  const kindHasResults = (slug: string) => {
    if (!search.trim()) return true;
    return !!filteredTagsByKind[slug];
  };

  const toggleKind = (slug: string) => {
    setExpandedKinds((prev) => {
      const next = new Set(prev);
      if (next.has(slug)) next.delete(slug);
      else next.add(slug);
      return next;
    });
  };

  const expandAll = () => {
    const all = [...lockedKinds, ...openKinds].map((k) => k.slug);
    setExpandedKinds(new Set(all));
  };

  const collapseAll = () => {
    setExpandedKinds(new Set());
  };

  if (kindsLoading || tagsLoading) {
    return <AdminLoader label="Loading tags..." />;
  }

  const totalTags = tagsData?.tags?.length || 0;
  const totalKinds = (kindsData?.tagKinds || []).length;

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-stone-900 mb-1">
            Tag Reference
          </h1>
          <p className="text-sm text-stone-500">
            {totalKinds} kinds &middot; {totalTags} total tags &middot;{" "}
            {lockedKinds.length} locked &middot; {openKinds.length} open
          </p>
        </div>

        {/* Search + controls */}
        <div className="flex items-center gap-3 mb-6">
          <div className="relative flex-1 max-w-sm">
            <Input
              type="text"
              placeholder="Search tags by value or name..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-9"
            />
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-stone-400" />
          </div>
          <button
            onClick={expandAll}
            className="text-xs text-stone-500 hover:text-stone-700 transition-colors"
          >
            Expand all
          </button>
          <span className="text-stone-300">&middot;</span>
          <button
            onClick={collapseAll}
            className="text-xs text-stone-500 hover:text-stone-700 transition-colors"
          >
            Collapse all
          </button>
        </div>

        {/* Locked tags section */}
        {lockedKinds.some((k) => kindHasResults(k.slug)) && (
          <section className="mb-8">
            <div className="flex items-center gap-2 mb-3">
              <h2 className="text-lg font-semibold text-stone-800">
                Locked
              </h2>
              <Badge variant="outline" className="text-[10px] uppercase tracking-wider">
                Fixed values
              </Badge>
            </div>
            <p className="text-xs text-stone-400 mb-4">
              Values are seeded and cannot be added or removed.
            </p>
            <div className="space-y-3">
              {lockedKinds
                .filter((k) => kindHasResults(k.slug))
                .map((kind) => (
                  <KindCard
                    key={kind.id}
                    kind={kind}
                    tags={filteredTagsByKind[kind.slug] || []}
                    allTags={tagsByKind[kind.slug] || []}
                    expanded={expandedKinds.has(kind.slug)}
                    onToggle={() => toggleKind(kind.slug)}
                    isFiltered={!!search.trim()}
                  />
                ))}
            </div>
          </section>
        )}

        {/* Open tags section */}
        {openKinds.some((k) => kindHasResults(k.slug)) && (
          <section className="mb-8">
            <div className="flex items-center gap-2 mb-3">
              <h2 className="text-lg font-semibold text-stone-800">
                Open
              </h2>
              <Badge variant="secondary" className="text-[10px] uppercase tracking-wider">
                Expandable
              </Badge>
            </div>
            <p className="text-xs text-stone-400 mb-4">
              New values can be added from individual post and source pages.
            </p>
            <div className="space-y-3">
              {openKinds
                .filter((k) => kindHasResults(k.slug))
                .map((kind) => (
                  <KindCard
                    key={kind.id}
                    kind={kind}
                    tags={filteredTagsByKind[kind.slug] || []}
                    allTags={tagsByKind[kind.slug] || []}
                    expanded={expandedKinds.has(kind.slug)}
                    onToggle={() => toggleKind(kind.slug)}
                    isFiltered={!!search.trim()}
                  />
                ))}
            </div>
          </section>
        )}

        {/* No results */}
        {search.trim() &&
          !lockedKinds.some((k) => kindHasResults(k.slug)) &&
          !openKinds.some((k) => kindHasResults(k.slug)) && (
            <div className="text-stone-400 text-center py-12">
              No tags matching &ldquo;{search}&rdquo;
            </div>
          )}
      </div>
    </div>
  );
}

// =============================================================================
// Kind Card — collapsible read-only card for each tag kind
// =============================================================================

function KindCard({
  kind,
  tags,
  allTags,
  expanded,
  onToggle,
  isFiltered,
}: {
  kind: TagKind;
  tags: Tag[];
  allTags: Tag[];
  expanded: boolean;
  onToggle: () => void;
  isFiltered: boolean;
}) {
  const resourceLabel = kind.allowedResourceTypes.join(", ");

  return (
    <div className="bg-white rounded-lg border border-stone-200 overflow-hidden">
      {/* Header — always visible */}
      <button
        onClick={onToggle}
        className="w-full flex items-center justify-between px-4 py-3 text-left hover:bg-stone-50/50 transition-colors"
      >
        <div className="flex items-center gap-2.5 min-w-0">
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-stone-400 shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-stone-400 shrink-0" />
          )}
          <span className="font-medium text-stone-900 truncate">
            {kind.displayName}
          </span>
          <code className="text-[11px] text-stone-400 bg-stone-100 px-1.5 py-0.5 rounded shrink-0">
            {kind.slug}
          </code>
          {kind.locked ? (
            <Badge variant="outline" className="text-[10px] shrink-0">
              <Lock className="h-3 w-3" />
              Locked
            </Badge>
          ) : (
            <Badge variant="secondary" className="text-[10px] shrink-0">
              <Pencil className="h-3 w-3" />
              Open
            </Badge>
          )}
          {kind.isPublic && (
            <Badge variant="success" className="text-[10px] shrink-0">
              <Eye className="h-3 w-3" />
              Public
            </Badge>
          )}
          {kind.required && (
            <Badge variant="warning" className="text-[10px] shrink-0">
              <AlertCircle className="h-3 w-3" />
              Required
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-3 shrink-0 ml-3">
          {resourceLabel && (
            <span className="text-[11px] text-stone-400 hidden sm:inline">
              {resourceLabel}
            </span>
          )}
          <span className="text-xs text-stone-500 tabular-nums">
            {isFiltered
              ? `${tags.length}/${allTags.length}`
              : allTags.length}{" "}
            tags
          </span>
        </div>
      </button>

      {/* Expanded content */}
      {expanded && (
        <div className="border-t border-stone-100 px-4 py-3">
          {/* Description */}
          {kind.description && (
            <p className="text-sm text-stone-500 mb-3">{kind.description}</p>
          )}

          {/* Resource types pill row */}
          {kind.allowedResourceTypes.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mb-3">
              <span className="text-[11px] text-stone-400 mr-1 self-center">
                Applies to:
              </span>
              {kind.allowedResourceTypes.map((rt) => (
                <Badge key={rt} variant="outline" className="text-[10px]">
                  {rt}
                </Badge>
              ))}
            </div>
          )}

          {/* Tag values */}
          {tags.length > 0 ? (
            <div className="flex flex-wrap gap-1.5">
              {tags.map((tag) => (
                <TagBadge key={tag.id} tag={tag} />
              ))}
            </div>
          ) : (
            <p className="text-sm text-stone-400 italic">
              {isFiltered ? "No matching tags" : "No tags yet"}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Tag Badge — compact display of a single tag value
// =============================================================================

function TagBadge({ tag }: { tag: Tag }) {
  const label = tag.displayName || tag.value;

  return (
    <Badge
      variant="secondary"
      color={tag.color || undefined}
      className="text-xs cursor-default"
      title={[
        tag.value,
        tag.description,
        tag.color ? `color: ${tag.color}` : null,
      ]
        .filter(Boolean)
        .join(" - ")}
    >
      {label}
    </Badge>
  );
}
