"use client";

import { useState, useMemo } from "react";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { TagKindsQuery, TagsQuery, CreateTagMutation } from "@/lib/graphql/tags";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Search,
  Lock,
  Pencil,
  Eye,
  AlertCircle,
  ChevronDown,
  ChevronRight,
  Plus,
} from "lucide-react";
import type { TagKind, Tag } from "@/gql/graphql";

export default function TagsPage() {
  const [{ data: kindsData, fetching: kindsLoading }] = useQuery({
    query: TagKindsQuery,
  });
  const [{ data: tagsData, fetching: tagsLoading }] = useQuery({
    query: TagsQuery,
  });
  const [, createTag] = useMutation(CreateTagMutation);
  const [search, setSearch] = useState("");
  const [expandedKinds, setExpandedKinds] = useState<Set<string>>(new Set());

  const handleCreateTag = async (kindSlug: string, value: string, displayName: string) => {
    const result = await createTag(
      { kind: kindSlug, value, displayName },
      { additionalTypenames: ["Tag"] },
    );
    if (result.error) throw result.error;
  };

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
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-foreground mb-1">
            Tag Reference
          </h1>
          <p className="text-sm text-muted-foreground">
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
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          </div>
          <Button
            variant="ghost"
            size="sm"
            onClick={expandAll}
            className="text-xs text-muted-foreground hover:text-foreground"
          >
            Expand all
          </Button>
          <span className="text-muted-foreground">&middot;</span>
          <Button
            variant="ghost"
            size="sm"
            onClick={collapseAll}
            className="text-xs text-muted-foreground hover:text-foreground"
          >
            Collapse all
          </Button>
        </div>

        {/* Locked tags section */}
        {lockedKinds.some((k) => kindHasResults(k.slug)) && (
          <section className="mb-8">
            <div className="flex items-center gap-2 mb-3">
              <h2 className="text-lg font-semibold text-foreground">
                Locked
              </h2>
              <Badge variant="outline" className="text-[10px] uppercase tracking-wider">
                Fixed values
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground mb-4">
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
              <h2 className="text-lg font-semibold text-foreground">
                Open
              </h2>
              <Badge variant="secondary" className="text-[10px] uppercase tracking-wider">
                Expandable
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground mb-4">
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
                    onCreateTag={(value, displayName) =>
                      handleCreateTag(kind.slug, value, displayName)
                    }
                  />
                ))}
            </div>
          </section>
        )}

        {/* No results */}
        {search.trim() &&
          !lockedKinds.some((k) => kindHasResults(k.slug)) &&
          !openKinds.some((k) => kindHasResults(k.slug)) && (
            <div className="text-muted-foreground text-center py-12">
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
  onCreateTag,
}: {
  kind: TagKind;
  tags: Tag[];
  allTags: Tag[];
  expanded: boolean;
  onToggle: () => void;
  isFiltered: boolean;
  onCreateTag?: (value: string, displayName: string) => Promise<void>;
}) {
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newValue, setNewValue] = useState("");
  const [newDisplayName, setNewDisplayName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  const handleCreate = async () => {
    if (!newValue.trim() || !onCreateTag) return;
    setIsCreating(true);
    setCreateError(null);
    try {
      await onCreateTag(newValue.trim(), newDisplayName.trim() || newValue.trim());
      setNewValue("");
      setNewDisplayName("");
      setShowCreateForm(false);
    } catch (err: any) {
      setCreateError(err.message || "Failed to create tag");
    } finally {
      setIsCreating(false);
    }
  };
  const resourceLabel = kind.allowedResourceTypes.join(", ");

  return (
    <div className="bg-card rounded-lg border border-border overflow-hidden">
      {/* Header — always visible */}
      <Button
        variant="ghost"
        onClick={onToggle}
        className="w-full flex items-center justify-between px-4 py-3 text-left h-auto rounded-none hover:bg-background/50"
      >
        <div className="flex items-center gap-2.5 min-w-0">
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          )}
          <span className="font-medium text-foreground truncate">
            {kind.displayName}
          </span>
          <code className="text-[11px] text-muted-foreground bg-muted px-1.5 py-0.5 rounded shrink-0">
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
            <span className="text-[11px] text-muted-foreground hidden sm:inline">
              {resourceLabel}
            </span>
          )}
          <span className="text-xs text-muted-foreground tabular-nums">
            {isFiltered
              ? `${tags.length}/${allTags.length}`
              : allTags.length}{" "}
            tags
          </span>
        </div>
      </Button>

      {/* Expanded content */}
      {expanded && (
        <div className="border-t border-border px-4 py-3">
          {/* Description */}
          {kind.description && (
            <p className="text-sm text-muted-foreground mb-3">{kind.description}</p>
          )}

          {/* Resource types pill row */}
          {kind.allowedResourceTypes.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mb-3">
              <span className="text-[11px] text-muted-foreground mr-1 self-center">
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
            <p className="text-sm text-muted-foreground italic">
              {isFiltered ? "No matching tags" : "No tags yet"}
            </p>
          )}

          {/* Create new tag form (open kinds only) */}
          {onCreateTag && (
            <div className="mt-3 border-t border-border pt-3">
              {showCreateForm ? (
                <div className="space-y-2">
                  <div className="flex items-end gap-2">
                    <div className="flex-1">
                      <label className="block text-[11px] text-muted-foreground mb-1">
                        Slug value
                      </label>
                      <Input
                        value={newValue}
                        onChange={(e) => setNewValue(e.target.value)}
                        placeholder="slug-value"
                        className="h-8 text-sm"
                        disabled={isCreating}
                        autoFocus
                      />
                    </div>
                    <div className="flex-1">
                      <label className="block text-[11px] text-muted-foreground mb-1">
                        Display Name
                      </label>
                      <Input
                        value={newDisplayName}
                        onChange={(e) => setNewDisplayName(e.target.value)}
                        placeholder="Display Name"
                        className="h-8 text-sm"
                        disabled={isCreating}
                      />
                    </div>
                    <Button
                      size="sm"
                      onClick={handleCreate}
                      disabled={!newValue.trim() || isCreating}
                      loading={isCreating}
                    >
                      Create
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={() => {
                        setShowCreateForm(false);
                        setNewValue("");
                        setNewDisplayName("");
                        setCreateError(null);
                      }}
                      disabled={isCreating}
                    >
                      Cancel
                    </Button>
                  </div>
                  {createError && (
                    <Alert variant="error" className="text-xs py-2 px-3">{createError}</Alert>
                  )}
                </div>
              ) : (
                <Button
                  variant="link"
                  size="sm"
                  onClick={() => setShowCreateForm(true)}
                  className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground px-0"
                >
                  <Plus className="h-3 w-3" />
                  Add new tag
                </Button>
              )}
            </div>
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
