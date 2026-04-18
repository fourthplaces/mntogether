"use client";

/**
 * AddNoteDialog — single source of truth for creating a note.
 *
 * Notes on this CMS must be attached to something (a post or an
 * organization) — the server's createNote mutation requires
 * noteableType + noteableId. So this dialog always renders scoped to
 * one entity. Two call sites today:
 *
 *   1. Post detail page Notes section → attached to that post.
 *   2. Org detail page Notes section → attached to that org.
 *
 * The /admin/notes list page's "New note" dialog layers an entity
 * picker on top (see NewNoteDialog) — it chooses what to attach to,
 * then delegates the actual field collection + submit to this same
 * component shape.
 *
 * Fields:
 *   - content (required) — the note body
 *   - severity: info | notice | urgent
 *   - isPublic — shown to readers, vs. internal/editorial-only
 *   - ctaText — optional call-to-action label for public notes
 *
 * sourceUrl is intentionally omitted here: that's a field the future
 * external-ingest path will populate; hand-created notes from the
 * admin don't need it.
 */

import * as React from "react";
import { useMutation } from "urql";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { CreateNoteMutation } from "@/lib/graphql/notes";

export type NoteableType = "post" | "organization";

export type AddNoteDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  noteableType: NoteableType;
  noteableId: string;
  /** Human-readable context for the dialog title (e.g. post title, org name). */
  entityLabel?: string;
  /** Fires after the server returns a new note. Use it to refetch the
   *  parent's notes list so the new row appears. */
  onCreated?: (noteId: string) => void;
};

const mutationContext = {
  additionalTypenames: ["Note", "NoteConnection"],
};

export function AddNoteDialog({
  open,
  onOpenChange,
  noteableType,
  noteableId,
  entityLabel,
  onCreated,
}: AddNoteDialogProps) {
  const [content, setContent] = React.useState("");
  const [severity, setSeverity] = React.useState<"info" | "notice" | "urgent">("info");
  const [isPublic, setIsPublic] = React.useState(false);
  const [ctaText, setCtaText] = React.useState("");
  const [error, setError] = React.useState<string | null>(null);

  const [{ fetching: saving }, createNote] = useMutation(CreateNoteMutation);

  // Reset fields whenever the dialog closes so reopening it doesn't
  // remember the last note. (Keep fields stable while open so a
  // re-render doesn't wipe what the user is typing.)
  React.useEffect(() => {
    if (!open) {
      setContent("");
      setSeverity("info");
      setIsPublic(false);
      setCtaText("");
      setError(null);
    }
  }, [open]);

  const canSubmit = content.trim().length > 0 && !saving;

  const handleSubmit = async () => {
    if (!canSubmit) return;
    setError(null);
    const result = await createNote(
      {
        noteableType,
        noteableId,
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

  const titleContext =
    noteableType === "post" ? "to post" : "to organization";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            Add note {titleContext}
            {entityLabel ? <span className="text-muted-foreground font-normal"> · {entityLabel}</span> : null}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="note-content">Content</Label>
            <Textarea
              id="note-content"
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
              <Label htmlFor="note-public">Visibility</Label>
              <div className="flex items-center gap-2 h-8">
                <Switch
                  id="note-public"
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
              <Label htmlFor="note-cta">Call to action (optional)</Label>
              <Input
                id="note-cta"
                value={ctaText}
                onChange={(e) => setCtaText(e.target.value)}
                placeholder="e.g. Learn more, Apply now"
                className="h-8 text-sm"
              />
            </div>
          )}

          {error && (
            <p className="text-sm text-danger-text">{error}</p>
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
