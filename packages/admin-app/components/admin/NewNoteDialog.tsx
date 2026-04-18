"use client";

/**
 * NewNoteDialog — "New note" from the /admin/notes list page.
 *
 * Unlike <AddNoteDialog> (which lives on a post or org detail page
 * and knows what it's attaching to), this dialog starts with no
 * target. The editor picks a post OR org first, then fills in the
 * note. Progressive-disclosure: fields appear once a target is
 * selected, and the target chip can be swapped out mid-flow.
 *
 * Used only from the notes list page. Everywhere else, prefer
 * <AddNoteDialog> directly — the entity is already in scope there.
 */

import * as React from "react";
import { useMutation, useQuery } from "urql";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Search, X, Building2, FileText } from "lucide-react";
import { cn } from "@/lib/utils";
import { CreateNoteMutation } from "@/lib/graphql/notes";
import { PostsListQuery } from "@/lib/graphql/posts";
import { OrganizationsListQuery } from "@/lib/graphql/organizations";

type NoteableType = "post" | "organization";

type Target = {
  type: NoteableType;
  id: string;
  label: string;
};

const mutationContext = { additionalTypenames: ["Note", "NoteConnection"] };

export function NewNoteDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: (noteId: string) => void;
}) {
  const [targetType, setTargetType] = React.useState<NoteableType>("post");
  const [target, setTarget] = React.useState<Target | null>(null);

  // Note fields
  const [content, setContent] = React.useState("");
  const [severity, setSeverity] = React.useState<"info" | "notice" | "urgent">("info");
  const [isPublic, setIsPublic] = React.useState(false);
  const [ctaText, setCtaText] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);

  const [{ fetching: saving }, createNote] = useMutation(CreateNoteMutation);

  // Reset on close.
  React.useEffect(() => {
    if (!open) {
      setTarget(null);
      setTargetType("post");
      setContent("");
      setSeverity("info");
      setIsPublic(false);
      setCtaText("");
      setError(null);
    }
  }, [open]);

  const canSubmit = target !== null && content.trim().length > 0 && !saving;

  const handleSubmit = async () => {
    if (!canSubmit || !target) return;
    setError(null);
    const result = await createNote(
      {
        noteableType: target.type,
        noteableId: target.id,
        content: content.trim(),
        severity,
        isPublic,
        ctaText: ctaText.trim() || null,
      },
      mutationContext,
    );
    if (result.error) {
      setError(result.error.message);
      return;
    }
    const newId = result.data?.createNote?.id;
    onOpenChange(false);
    if (newId) onCreated?.(newId);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>New note</DialogTitle>
        </DialogHeader>

        <div className="space-y-3">
          {/* Step 1: target picker (collapses to a chip once selected) */}
          {target ? (
            <div className="flex items-center gap-2 rounded-md border border-border bg-secondary/40 px-2.5 py-1.5">
              {target.type === "post" ? (
                <FileText className="size-4 text-muted-foreground shrink-0" />
              ) : (
                <Building2 className="size-4 text-muted-foreground shrink-0" />
              )}
              <div className="flex-1 min-w-0">
                <div className="text-[11px] uppercase tracking-wide text-muted-foreground">
                  {target.type === "post" ? "Post" : "Organization"}
                </div>
                <div className="text-sm text-foreground truncate">{target.label}</div>
              </div>
              <Button
                variant="ghost"
                size="icon-xs"
                onClick={() => setTarget(null)}
                title="Change target"
              >
                <X className="size-3.5" />
              </Button>
            </div>
          ) : (
            <TargetPicker
              targetType={targetType}
              onTargetTypeChange={setTargetType}
              onPick={setTarget}
            />
          )}

          {/* Step 2: fields — only shown once a target is chosen so the
           *          dialog doesn't overwhelm on open. */}
          {target && (
            <>
              <div className="space-y-1.5">
                <Label htmlFor="new-note-content">Content</Label>
                <Textarea
                  id="new-note-content"
                  value={content}
                  onChange={(e) => setContent(e.target.value)}
                  placeholder="What's the note about?"
                  rows={4}
                  autoFocus
                />
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div className="space-y-1.5">
                  <Label>Severity</Label>
                  <Select
                    value={severity}
                    onValueChange={(v) =>
                      v != null && setSeverity(v as "info" | "notice" | "urgent")
                    }
                  >
                    <SelectTrigger className="h-8 text-sm">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="info">Info</SelectItem>
                      <SelectItem value="notice">Notice</SelectItem>
                      <SelectItem value="urgent">Urgent</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-1.5">
                  <Label htmlFor="new-note-public">Visibility</Label>
                  <div className="flex items-center gap-2 h-8">
                    <Switch
                      id="new-note-public"
                      checked={isPublic}
                      onCheckedChange={setIsPublic}
                    />
                    <span className="text-sm text-muted-foreground">
                      {isPublic ? "Public" : "Internal"}
                    </span>
                  </div>
                </div>
              </div>

              {isPublic && (
                <div className="space-y-1.5">
                  <Label htmlFor="new-note-cta">Call to action (optional)</Label>
                  <Input
                    id="new-note-cta"
                    value={ctaText}
                    onChange={(e) => setCtaText(e.target.value)}
                    placeholder="e.g. Learn more, Apply now"
                    className="h-8 text-sm"
                  />
                </div>
              )}

              {error && <p className="text-sm text-danger-text">{error}</p>}
            </>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={saving}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!canSubmit} loading={saving}>
            Add note
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// TargetPicker — type toggle + search-backed list
// ---------------------------------------------------------------------------

function TargetPicker({
  targetType,
  onTargetTypeChange,
  onPick,
}: {
  targetType: NoteableType;
  onTargetTypeChange: (t: NoteableType) => void;
  onPick: (target: Target) => void;
}) {
  const [search, setSearch] = React.useState("");
  const debouncedSearch = useDebounced(search, 200);

  // Post search — only fires when we're in post mode and the user's
  // typed enough to make the query worth running.
  const [{ data: postsData, fetching: postsFetching }] = useQuery({
    query: PostsListQuery,
    variables: { search: debouncedSearch, limit: 10 },
    pause: targetType !== "post" || debouncedSearch.length < 2,
  });

  // Org list — fetched in full once when the user switches to org mode;
  // small list so no server-side search is needed.
  const [{ data: orgsData, fetching: orgsFetching }] = useQuery({
    query: OrganizationsListQuery,
    pause: targetType !== "organization",
  });

  const postResults = postsData?.posts?.posts ?? [];
  const orgResults = (orgsData?.organizations ?? []).filter((o) =>
    debouncedSearch.length === 0
      ? true
      : o.name.toLowerCase().includes(debouncedSearch.toLowerCase()),
  );

  return (
    <div className="space-y-2">
      <Tabs value={targetType} onValueChange={(v) => onTargetTypeChange(v as NoteableType)}>
        <TabsList>
          <TabsTrigger value="post">
            <FileText className="size-3.5" /> Post
          </TabsTrigger>
          <TabsTrigger value="organization">
            <Building2 className="size-3.5" /> Organization
          </TabsTrigger>
        </TabsList>
      </Tabs>

      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
        <Input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={
            targetType === "post"
              ? "Search posts by title…"
              : "Filter organizations…"
          }
          className="pl-8 text-sm"
          autoFocus
        />
      </div>

      <div className={cn("max-h-60 overflow-y-auto -mx-1", "rounded-md border border-border")}>
        {targetType === "post" ? (
          debouncedSearch.length < 2 ? (
            <EmptyHint>Type at least 2 characters to search posts.</EmptyHint>
          ) : postsFetching ? (
            <EmptyHint>Searching…</EmptyHint>
          ) : postResults.length === 0 ? (
            <EmptyHint>No matching posts.</EmptyHint>
          ) : (
            <ResultList>
              {postResults.map((p) => (
                <ResultRow
                  key={p.id}
                  icon={<FileText className="size-3.5 text-muted-foreground" />}
                  label={p.title}
                  onClick={() => onPick({ type: "post", id: p.id, label: p.title })}
                />
              ))}
            </ResultList>
          )
        ) : orgsFetching && orgResults.length === 0 ? (
          <EmptyHint>Loading organizations…</EmptyHint>
        ) : orgResults.length === 0 ? (
          <EmptyHint>No organizations found.</EmptyHint>
        ) : (
          <ResultList>
            {orgResults.map((o) => (
              <ResultRow
                key={o.id}
                icon={<Building2 className="size-3.5 text-muted-foreground" />}
                label={o.name}
                onClick={() =>
                  onPick({ type: "organization", id: o.id, label: o.name })
                }
              />
            ))}
          </ResultList>
        )}
      </div>
    </div>
  );
}

function EmptyHint({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-sm text-muted-foreground italic px-3 py-3">{children}</p>
  );
}

function ResultList({ children }: { children: React.ReactNode }) {
  return <div className="py-1">{children}</div>;
}

function ResultRow({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="w-full text-left flex items-center gap-2 px-3 py-1.5 hover:bg-accent/60 transition-colors"
    >
      {icon}
      <span className="text-sm text-foreground truncate">{label}</span>
    </button>
  );
}

function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = React.useState(value);
  React.useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}
