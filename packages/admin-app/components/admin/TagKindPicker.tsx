"use client";

import { useState, useMemo } from "react";
import { Check, Plus, Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface PickerTag {
  id: string;
  value: string;
  displayName?: string | null;
  color?: string | null;
}

export interface TagKindPickerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  kindSlug: string;
  kindDisplayName: string;
  locked: boolean;
  /** Values already applied to the entity — these are excluded from the grid */
  appliedTagValues: Set<string>;
  /** All tags for this kind (unfiltered) */
  allTags: PickerTag[];
  /** Called with the selected tags when user confirms */
  onConfirm: (
    selections: Array<{ value: string; displayName: string }>
  ) => Promise<void>;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

const SEARCH_THRESHOLD = 15;

export function TagKindPicker({
  open,
  onOpenChange,
  kindDisplayName,
  locked,
  appliedTagValues,
  allTags,
  onConfirm,
}: TagKindPickerProps) {
  const [selectedValues, setSelectedValues] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  // "Create new" state (open kinds only)
  const [newValue, setNewValue] = useState("");
  const [newDisplayName, setNewDisplayName] = useState("");
  const [createdTags, setCreatedTags] = useState<
    Array<{ value: string; displayName: string }>
  >([]);

  // Tags not yet applied to the entity
  const pickableTags = useMemo(
    () => allTags.filter((t) => !appliedTagValues.has(t.value)),
    [allTags, appliedTagValues]
  );

  // Filtered by search
  const filteredTags = useMemo(() => {
    if (!searchQuery.trim()) return pickableTags;
    const q = searchQuery.toLowerCase();
    return pickableTags.filter(
      (t) =>
        t.value.toLowerCase().includes(q) ||
        t.displayName?.toLowerCase().includes(q)
    );
  }, [pickableTags, searchQuery]);

  const showSearch = pickableTags.length > SEARCH_THRESHOLD;
  const selectedCount = selectedValues.size + createdTags.length;

  const toggleValue = (value: string) => {
    setSelectedValues((prev) => {
      const next = new Set(prev);
      if (next.has(value)) next.delete(value);
      else next.add(value);
      return next;
    });
  };

  const handleCreateNew = () => {
    if (!newValue.trim()) return;
    const val = newValue.trim();
    const display = newDisplayName.trim() || val;
    // Add to created list and auto-select
    setCreatedTags((prev) => [...prev, { value: val, displayName: display }]);
    setNewValue("");
    setNewDisplayName("");
  };

  const removeCreated = (value: string) => {
    setCreatedTags((prev) => prev.filter((t) => t.value !== value));
  };

  const handleConfirm = async () => {
    setIsSubmitting(true);
    try {
      // Combine existing-tag selections + newly-created tags
      const fromExisting: Array<{ value: string; displayName: string }> = [];
      for (const val of selectedValues) {
        const tag = pickableTags.find((t) => t.value === val);
        fromExisting.push({
          value: val,
          displayName: tag?.displayName || val,
        });
      }
      await onConfirm([...fromExisting, ...createdTags]);
      handleReset();
      onOpenChange(false);
    } catch (err) {
      console.error("Failed to add tags:", err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleReset = () => {
    setSelectedValues(new Set());
    setSearchQuery("");
    setNewValue("");
    setNewDisplayName("");
    setCreatedTags([]);
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(val) => {
        if (!val) handleReset();
        onOpenChange(val);
      }}
    >
      <DialogContent className="max-w-lg max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>Add {kindDisplayName}</DialogTitle>
        </DialogHeader>

        {/* Search */}
        {showSearch && (
          <div className="relative">
            <Input
              type="text"
              placeholder={`Search ${kindDisplayName.toLowerCase()}...`}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          </div>
        )}

        {/* Badge grid */}
        <div className="flex flex-wrap gap-2 max-h-[50vh] overflow-y-auto py-1">
          {filteredTags.length === 0 && createdTags.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4">
              {searchQuery
                ? "No matching tags"
                : "All values are already applied"}
            </p>
          ) : (
            <>
              {filteredTags.map((tag) => {
                const isSelected = selectedValues.has(tag.value);
                return (
                  <button
                    key={tag.value}
                    type="button"
                    onClick={() => toggleValue(tag.value)}
                    className={cn(
                      "inline-flex items-center rounded-full px-3 py-1.5 text-sm font-medium border transition-colors cursor-pointer",
                      isSelected
                        ? "bg-primary/10 border-primary text-primary"
                        : "bg-secondary border-transparent text-secondary-foreground hover:bg-accent"
                    )}
                  >
                    {isSelected && (
                      <Check className="w-3.5 h-3.5 mr-1.5 shrink-0" />
                    )}
                    {tag.displayName || tag.value}
                  </button>
                );
              })}

              {/* Show newly created tags in the grid too */}
              {createdTags.map((tag) => (
                <button
                  key={`new-${tag.value}`}
                  type="button"
                  onClick={() => removeCreated(tag.value)}
                  className="inline-flex items-center rounded-full px-3 py-1.5 text-sm font-medium border transition-colors cursor-pointer bg-primary/10 border-primary text-primary"
                >
                  <Check className="w-3.5 h-3.5 mr-1.5 shrink-0" />
                  {tag.displayName}
                  <span className="text-xs ml-1 opacity-60">(new)</span>
                </button>
              ))}
            </>
          )}
        </div>

        {/* Create new section (open kinds only) */}
        {!locked && (
          <div className="border-t border-border pt-3">
            <p className="text-xs text-muted-foreground mb-2">
              Create a new tag value
            </p>
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <Input
                  value={newValue}
                  onChange={(e) => setNewValue(e.target.value)}
                  placeholder="slug-value"
                  className="h-8 text-sm"
                />
              </div>
              <div className="flex-1">
                <Input
                  value={newDisplayName}
                  onChange={(e) => setNewDisplayName(e.target.value)}
                  placeholder="Display Name"
                  className="h-8 text-sm"
                />
              </div>
              <Button
                size="sm"
                variant="outline"
                onClick={handleCreateNew}
                disabled={!newValue.trim()}
              >
                <Plus className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        )}

        {/* Footer */}
        <DialogFooter className="flex items-center">
          <span className="text-sm text-muted-foreground mr-auto">
            {selectedCount > 0
              ? `${selectedCount} selected`
              : "Click tags to select"}
          </span>
          <Button
            variant="ghost"
            onClick={() => onOpenChange(false)}
            disabled={isSubmitting}
          >
            Cancel
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={selectedCount === 0 || isSubmitting}
            loading={isSubmitting}
          >
            Add{selectedCount > 0 ? ` ${selectedCount}` : ""}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
