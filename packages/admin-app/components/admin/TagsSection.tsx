"use client";

import { useState, useMemo } from "react";
import { Plus, X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { TagKindPicker, type PickerTag } from "./TagKindPicker";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AppliedTag {
  id: string;
  kind: string;
  value: string;
  displayName?: string | null;
  color?: string | null;
}

interface TagKindMeta {
  slug: string;
  displayName: string;
  locked: boolean;
}

export interface TagsSectionProps {
  /** Tags currently applied to the entity */
  tags: readonly AppliedTag[];
  /** Tag kinds applicable to this resource type */
  applicableKinds: readonly TagKindMeta[];
  /** All tag values grouped by kind slug */
  allTagsByKind: Record<string, PickerTag[]>;
  /** Remove a single tag by ID */
  onRemoveTag: (tagId: string) => Promise<void>;
  /** Add tags of a given kind */
  onAddTags: (
    kindSlug: string,
    tags: Array<{ value: string; displayName: string }>
  ) => Promise<void>;
  /** Whether mutations are in progress */
  disabled?: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function TagsSection({
  tags,
  applicableKinds,
  allTagsByKind,
  onRemoveTag,
  onAddTags,
  disabled,
}: TagsSectionProps) {
  const [pickerKind, setPickerKind] = useState<string | null>(null);
  const [removingId, setRemovingId] = useState<string | null>(null);

  // Group applied tags by kind
  const tagsByKind = useMemo(() => {
    const map: Record<string, AppliedTag[]> = {};
    for (const tag of tags) {
      if (!map[tag.kind]) map[tag.kind] = [];
      map[tag.kind].push(tag);
    }
    return map;
  }, [tags]);

  // Get applied values for the currently open picker
  const appliedValuesForPicker = useMemo(() => {
    if (!pickerKind) return new Set<string>();
    return new Set((tagsByKind[pickerKind] || []).map((t) => t.value));
  }, [pickerKind, tagsByKind]);

  const pickerMeta = applicableKinds.find((k) => k.slug === pickerKind);

  const handleRemove = async (tagId: string) => {
    setRemovingId(tagId);
    try {
      await onRemoveTag(tagId);
    } finally {
      setRemovingId(null);
    }
  };

  return (
    <div className="border-t border-border pt-4">
      <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-3">
        Tags
      </h3>

      <div className="space-y-3">
        {applicableKinds.map((kind) => {
          const kindTags = tagsByKind[kind.slug] || [];

          return (
            <div key={kind.slug}>
              {/* Kind header row */}
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs text-muted-foreground uppercase font-medium">
                  {kind.displayName}
                </span>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  onClick={() => setPickerKind(kind.slug)}
                  disabled={disabled}
                  title={`Add ${kind.displayName.toLowerCase()}`}
                >
                  <Plus className="h-3.5 w-3.5" />
                </Button>
              </div>

              {/* Applied tag badges */}
              {kindTags.length > 0 ? (
                <div className="flex flex-wrap gap-1.5">
                  {kindTags.map((tag) => (
                    <Badge
                      key={tag.id}
                      variant="secondary"
                      color={tag.color || undefined}
                      className="gap-1 pr-1 text-[11px]"
                    >
                      {tag.displayName || tag.value}
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        onClick={() => handleRemove(tag.id)}
                        disabled={disabled || removingId === tag.id}
                        className="size-4 rounded-sm hover:text-destructive hover:bg-destructive/10 ml-0.5"
                      >
                        <X className="w-3 h-3" />
                      </Button>
                    </Badge>
                  ))}
                </div>
              ) : (
                <p className="text-xs text-muted-foreground/60 italic">none</p>
              )}
            </div>
          );
        })}
      </div>

      {/* Picker dialog */}
      {pickerKind && pickerMeta && (
        <TagKindPicker
          open
          onOpenChange={(open) => {
            if (!open) setPickerKind(null);
          }}
          kindSlug={pickerKind}
          kindDisplayName={pickerMeta.displayName}
          locked={pickerMeta.locked}
          appliedTagValues={appliedValuesForPicker}
          allTags={allTagsByKind[pickerKind] || []}
          onConfirm={(selections) => onAddTags(pickerKind, selections)}
        />
      )}
    </div>
  );
}
