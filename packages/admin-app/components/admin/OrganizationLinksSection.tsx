"use client";

/**
 * OrganizationLinksSection
 * -------------------------------------------------------------------------
 * Sidebar editor for an org/source's external profile URLs. Replaces the
 * old Platform tag kind with first-class (platform, url, is_public) rows.
 *
 * UX model — read-first. The narrow right sidebar can't support
 * per-row inline form fields without becoming cramped, so we keep the
 * list compact and defer editing into a Dialog:
 *
 *   - Each link renders as a single tappable row (platform name + the
 *     host portion of the URL). Click to edit.
 *   - "Add link" button opens the same Dialog in create mode.
 *   - Visibility (eye icon) and reorder (up/down arrows) stay inline
 *     because both are one-click operations and useful at a glance;
 *     the parent row click handler ignores clicks that originate
 *     inside those buttons.
 *
 * Default visibility when creating:
 *   - source_type = "organization" → public
 *   - source_type = "individual"   → hidden
 * Enforced server-side when isPublic is omitted; the dialog's toggle
 * defaults to the matching value so the editor sees what will happen.
 *
 * The `platform` slug matches `tags.value` where `tags.kind='platform'`.
 * We read that tags row for display metadata (name, emoji, color).
 */

import { useEffect, useMemo, useState } from "react";
import { useQuery, useMutation } from "urql";
import {
  ChevronDown,
  ChevronUp,
  ExternalLink,
  Eye,
  EyeOff,
  Plus,
  Trash2,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";

import {
  PlatformOptionsQuery,
  UpsertOrganizationLinkMutation,
  DeleteOrganizationLinkMutation,
  ReorderOrganizationLinksMutation,
} from "@/lib/graphql/organizations";

const orgMutationContext = {
  additionalTypenames: ["Organization", "OrganizationLink"],
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface OrgLink {
  id: string;
  organizationId: string;
  platform: string;
  url: string;
  isPublic: boolean;
  displayOrder: number;
}

interface PlatformOption {
  id: string;
  value: string;
  displayName: string | null;
  emoji: string | null;
  color: string | null;
}

interface Props {
  organizationId: string;
  /** "organization" or "individual" — drives the default visibility on
   * new-link creation. Server-side is authoritative; the dialog uses
   * this for its initial toggle state so editors see what will happen. */
  sourceType: string;
  links: readonly OrgLink[];
  onChanged: () => void;
}

// ---------------------------------------------------------------------------
// Main section
// ---------------------------------------------------------------------------

export function OrganizationLinksSection({
  organizationId,
  sourceType,
  links,
  onChanged,
}: Props) {
  const [{ data: platformsData }] = useQuery({ query: PlatformOptionsQuery });
  const platforms = useMemo<PlatformOption[]>(
    () => (platformsData?.tags ?? []) as PlatformOption[],
    [platformsData]
  );
  const platformMap = useMemo(() => {
    const m: Record<string, PlatformOption> = {};
    for (const p of platforms) m[p.value] = p;
    return m;
  }, [platforms]);

  const [, upsertLink] = useMutation(UpsertOrganizationLinkMutation);
  const [, deleteLink] = useMutation(DeleteOrganizationLinkMutation);
  const [, reorderLinks] = useMutation(ReorderOrganizationLinksMutation);

  const [editing, setEditing] = useState<OrgLink | "new" | null>(null);
  const [pendingId, setPendingId] = useState<string | null>(null);

  // ------------------------------------------------------------------
  // Inline operations — don't open the dialog
  // ------------------------------------------------------------------

  const toggleVisibility = async (link: OrgLink) => {
    setPendingId(link.id);
    try {
      const res = await upsertLink(
        {
          linkId: link.id,
          organizationId,
          platform: link.platform,
          url: link.url,
          isPublic: !link.isPublic,
        },
        orgMutationContext
      );
      if (res.error) throw res.error;
      onChanged();
    } catch (err) {
      console.error("Failed to toggle visibility:", err);
    } finally {
      setPendingId(null);
    }
  };

  const moveLink = async (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= links.length) return;
    const reordered = [...links];
    [reordered[index], reordered[target]] = [reordered[target], reordered[index]];
    setPendingId(links[index].id);
    try {
      const res = await reorderLinks(
        { organizationId, linkIds: reordered.map((l) => l.id) },
        orgMutationContext
      );
      if (res.error) throw res.error;
      onChanged();
    } catch (err) {
      console.error("Failed to reorder:", err);
    } finally {
      setPendingId(null);
    }
  };

  // ------------------------------------------------------------------
  // Dialog operations
  // ------------------------------------------------------------------

  const handleSave = async (values: {
    platform: string;
    url: string;
    isPublic: boolean;
  }) => {
    const linkId = editing && editing !== "new" ? editing.id : null;
    setPendingId(linkId ?? "__new__");
    try {
      const res = await upsertLink(
        {
          linkId,
          organizationId,
          platform: values.platform,
          url: values.url,
          isPublic: values.isPublic,
        },
        orgMutationContext
      );
      if (res.error) throw res.error;
      setEditing(null);
      onChanged();
    } catch (err) {
      console.error("Failed to save link:", err);
    } finally {
      setPendingId(null);
    }
  };

  const handleDelete = async (link: OrgLink) => {
    setPendingId(link.id);
    try {
      const res = await deleteLink({ linkId: link.id }, orgMutationContext);
      if (res.error) throw res.error;
      setEditing(null);
      onChanged();
    } catch (err) {
      console.error("Failed to delete link:", err);
    } finally {
      setPendingId(null);
    }
  };

  // ------------------------------------------------------------------
  // Render
  // ------------------------------------------------------------------

  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Links{links.length > 0 ? ` (${links.length})` : ""}
        </h3>
        <Button variant="outline" size="xs" onClick={() => setEditing("new")}>
          <Plus className="size-3 mr-1" /> Add link
        </Button>
      </div>

      {links.length === 0 ? (
        <p className="text-sm text-text-faint italic">
          No platform links yet.
        </p>
      ) : (
        <ul className="space-y-1">
          {links.map((link, i) => (
            <LinkRow
              key={link.id}
              link={link}
              meta={platformMap[link.platform]}
              isFirst={i === 0}
              isLast={i === links.length - 1}
              pending={pendingId === link.id}
              disabled={pendingId !== null && pendingId !== link.id}
              onOpen={() => setEditing(link)}
              onToggleVisibility={() => toggleVisibility(link)}
              onMoveUp={() => moveLink(i, -1)}
              onMoveDown={() => moveLink(i, 1)}
            />
          ))}
        </ul>
      )}

      <LinkDialog
        open={editing !== null}
        mode={editing === "new" ? "create" : "edit"}
        link={editing && editing !== "new" ? editing : null}
        platforms={platforms}
        defaultIsPublic={sourceType !== "individual"}
        saving={pendingId === "__new__" || (editing && editing !== "new" && pendingId === editing.id) || false}
        onClose={() => setEditing(null)}
        onSave={handleSave}
        onDelete={handleDelete}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Read-state row
// ---------------------------------------------------------------------------

interface LinkRowProps {
  link: OrgLink;
  meta: PlatformOption | undefined;
  isFirst: boolean;
  isLast: boolean;
  pending: boolean;
  disabled: boolean;
  onOpen: () => void;
  onToggleVisibility: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

function LinkRow({
  link,
  meta,
  isFirst,
  isLast,
  pending,
  disabled,
  onOpen,
  onToggleVisibility,
  onMoveUp,
  onMoveDown,
}: LinkRowProps) {
  const platformLabel = meta?.displayName ?? link.platform;
  const displayUrl = prettifyUrl(link.url);

  return (
    <li>
      <div
        className={`group flex items-center gap-1.5 p-2 rounded-md border border-border bg-card hover:border-foreground/30 transition-colors ${
          pending ? "opacity-60" : ""
        }`}
      >
        {/* Reorder — small, stacked, minimal visual weight */}
        <div className="flex flex-col -space-y-0.5 shrink-0">
          <button
            type="button"
            className="text-muted-foreground/60 hover:text-foreground disabled:opacity-20 leading-none p-0.5"
            onClick={(e) => {
              e.stopPropagation();
              onMoveUp();
            }}
            disabled={disabled || isFirst || pending}
            aria-label="Move up"
            title="Move up"
          >
            <ChevronUp className="size-3" />
          </button>
          <button
            type="button"
            className="text-muted-foreground/60 hover:text-foreground disabled:opacity-20 leading-none p-0.5"
            onClick={(e) => {
              e.stopPropagation();
              onMoveDown();
            }}
            disabled={disabled || isLast || pending}
            aria-label="Move down"
            title="Move down"
          >
            <ChevronDown className="size-3" />
          </button>
        </div>

        {/* Main clickable area — opens the edit dialog */}
        <button
          type="button"
          onClick={onOpen}
          disabled={disabled || pending}
          className="flex-1 min-w-0 flex items-center gap-2 text-left"
        >
          {meta?.emoji ? (
            <span className="shrink-0 text-base leading-none" aria-hidden>
              {meta.emoji}
            </span>
          ) : null}
          <div className="min-w-0 flex-1">
            <div className="text-sm font-medium text-foreground truncate leading-tight">
              {platformLabel}
            </div>
            <div className="text-xs text-muted-foreground truncate leading-tight">
              {displayUrl}
            </div>
          </div>
        </button>

        {/* Inline visibility toggle */}
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onToggleVisibility();
          }}
          disabled={disabled || pending}
          aria-label={
            link.isPublic ? "Hide from public profile" : "Show on public profile"
          }
          title={
            link.isPublic ? "Visible on public profile" : "Hidden — CMS only"
          }
          className="shrink-0 p-1 rounded hover:bg-accent disabled:opacity-40"
        >
          {link.isPublic ? (
            <Eye className="size-3.5 text-foreground" />
          ) : (
            <EyeOff className="size-3.5 text-muted-foreground" />
          )}
        </button>
      </div>
    </li>
  );
}

/** Show just the host + path without protocol / trailing slash. Keeps
 *  the sidebar row readable even with a long Substack URL. */
function prettifyUrl(url: string): string {
  try {
    const u = new URL(url);
    const tail = (u.pathname + u.search).replace(/\/$/, "");
    return (u.host + tail).replace(/^www\./, "");
  } catch {
    return url;
  }
}

// ---------------------------------------------------------------------------
// Edit/Create dialog
// ---------------------------------------------------------------------------

interface LinkDialogProps {
  open: boolean;
  mode: "create" | "edit";
  link: OrgLink | null;
  platforms: PlatformOption[];
  defaultIsPublic: boolean;
  saving: boolean;
  onClose: () => void;
  onSave: (values: {
    platform: string;
    url: string;
    isPublic: boolean;
  }) => void;
  onDelete: (link: OrgLink) => void;
}

function LinkDialog({
  open,
  mode,
  link,
  platforms,
  defaultIsPublic,
  saving,
  onClose,
  onSave,
  onDelete,
}: LinkDialogProps) {
  // `platform` is null (not "") when nothing is selected, so we can pass
  // it through to Base UI's Select as a controlled value from first render.
  // Switching a Select between `value={undefined}` and `value="facebook"`
  // triggers an uncontrolled→controlled warning.
  const [platform, setPlatform] = useState<string | null>(null);
  const [url, setUrl] = useState("");
  const [isPublic, setIsPublic] = useState(defaultIsPublic);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  // Sync dialog inputs whenever the caller opens the dialog or swaps
  // which link is being edited. (Dialogs stay mounted; we can't rely
  // on initial state.)
  useEffect(() => {
    if (!open) return;
    setConfirmingDelete(false);
    if (link) {
      setPlatform(link.platform);
      setUrl(link.url);
      setIsPublic(link.isPublic);
    } else {
      setPlatform(null);
      setUrl("");
      setIsPublic(defaultIsPublic);
    }
  }, [open, link, defaultIsPublic]);

  const canSave = Boolean(platform && url.trim());

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSave || !platform) return;
    onSave({ platform, url: url.trim(), isPublic });
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {mode === "create" ? "Add link" : "Edit link"}
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Platform */}
          <div className="space-y-1.5">
            <Label htmlFor="link-platform">Platform</Label>
            <Select
              value={platform}
              onValueChange={(v) => setPlatform(v)}
              disabled={saving}
            >
              <SelectTrigger id="link-platform" className="w-full">
                <SelectValue placeholder="Choose a platform…" />
              </SelectTrigger>
              <SelectContent>
                {platforms.map((p) => (
                  <SelectItem key={p.value} value={p.value}>
                    <span className="inline-flex items-center gap-2">
                      {p.emoji && <span>{p.emoji}</span>}
                      <span>{p.displayName ?? p.value}</span>
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* URL */}
          <div className="space-y-1.5">
            <Label htmlFor="link-url">URL</Label>
            <Input
              id="link-url"
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://…"
              disabled={saving}
              autoFocus={mode === "edit"}
            />
          </div>

          {/* Visibility */}
          <div className="flex items-start justify-between gap-3 rounded-lg border border-border p-3">
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-foreground flex items-center gap-1.5">
                {isPublic ? (
                  <Eye className="size-3.5 text-foreground" />
                ) : (
                  <EyeOff className="size-3.5 text-muted-foreground" />
                )}
                {isPublic ? "Visible on public profile" : "Hidden — CMS only"}
              </div>
              <div className="text-xs text-muted-foreground mt-0.5">
                {isPublic
                  ? "Readers can click through to this link from the public profile page."
                  : "Editors can see this link in the CMS, but it won't appear on the public profile."}
              </div>
            </div>
            <Switch
              checked={isPublic}
              onCheckedChange={setIsPublic}
              disabled={saving}
              aria-label="Visible on public profile"
            />
          </div>

          <DialogFooter className="flex items-center justify-between sm:justify-between gap-2">
            {/* Delete lives in the footer-left for edit mode */}
            <div>
              {mode === "edit" && link && (
                !confirmingDelete ? (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="text-destructive hover:text-destructive hover:bg-destructive/10"
                    onClick={() => setConfirmingDelete(true)}
                    disabled={saving}
                  >
                    <Trash2 className="size-3.5 mr-1" /> Delete
                  </Button>
                ) : (
                  <div className="flex items-center gap-1.5">
                    <span className="text-xs text-muted-foreground">
                      Delete this link?
                    </span>
                    <Button
                      type="button"
                      variant="destructive"
                      size="sm"
                      onClick={() => onDelete(link)}
                      disabled={saving}
                    >
                      Confirm
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setConfirmingDelete(false)}
                      disabled={saving}
                    >
                      Cancel
                    </Button>
                  </div>
                )
              )}
            </div>

            <div className="flex items-center gap-2">
              {mode === "edit" && url && (
                <a
                  href={url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-xs text-muted-foreground hover:text-foreground inline-flex items-center gap-1"
                >
                  Open <ExternalLink className="size-3" />
                </a>
              )}
              <Button
                type="button"
                variant="ghost"
                onClick={onClose}
                disabled={saving}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={!canSave || saving} loading={saving}>
                {mode === "create" ? "Add" : "Save"}
              </Button>
            </div>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
