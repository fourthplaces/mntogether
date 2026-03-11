"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
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
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from "@/components/ui/dropdown-menu";
import {
  NoteDetailQuery,
  UpdateNoteMutation,
  DeleteNoteMutation,
  LinkNoteMutation,
  UnlinkNoteMutation,
} from "@/lib/graphql/notes";
import { PostsListQuery } from "@/lib/graphql/posts";
import { OrganizationsListQuery } from "@/lib/graphql/organizations";
import { ArrowLeft, ExternalLink, Plus, X } from "lucide-react";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const SEVERITY_BADGE_VARIANT: Record<string, "danger" | "warning" | "info"> = {
  urgent: "danger",
  notice: "warning",
  info: "info",
};

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleString();
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-3">
      {children}
    </h3>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function NoteDetailPage() {
  const params = useParams();
  const router = useRouter();
  const noteId = params.id as string;

  // ─── Form state ───────────────────────────────────────────────
  const [content, setContent] = useState("");
  const [severity, setSeverity] = useState("info");
  const [isPublic, setIsPublic] = useState(false);
  const [ctaText, setCtaText] = useState("");
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [saving, setSaving] = useState(false);
  const [initialized, setInitialized] = useState(false);

  // Link dialogs
  const [showLinkPostDialog, setShowLinkPostDialog] = useState(false);
  const [showLinkOrgDialog, setShowLinkOrgDialog] = useState(false);
  const [postSearch, setPostSearch] = useState("");

  // ─── Queries & Mutations ──────────────────────────────────────
  const [{ data, fetching, error }, reexecuteQuery] = useQuery({
    query: NoteDetailQuery,
    variables: { id: noteId },
  });

  const [, updateNote] = useMutation(UpdateNoteMutation);
  const [, deleteNote] = useMutation(DeleteNoteMutation);
  const [, linkNote] = useMutation(LinkNoteMutation);
  const [, unlinkNote] = useMutation(UnlinkNoteMutation);

  const note = data?.note;

  // Post search query — only runs when dialog is open and search has text
  const [{ data: postsData, fetching: postsFetching }] = useQuery({
    query: PostsListQuery,
    variables: { search: postSearch, limit: 10 },
    pause: !showLinkPostDialog || postSearch.length < 2,
  });

  // Org list — only runs when dialog is open
  const [{ data: orgsData }] = useQuery({
    query: OrganizationsListQuery,
    pause: !showLinkOrgDialog,
  });

  // Seed form state from fetched data
  useEffect(() => {
    if (note && !initialized) {
      setContent(note.content);
      setSeverity(note.severity);
      setIsPublic(note.isPublic);
      setCtaText(note.ctaText || "");
      setInitialized(true);
    }
  }, [note, initialized]);

  // ─── Derived data ─────────────────────────────────────────────
  const linkedPostIds = new Set(note?.linkedPosts?.map((p) => p.id) ?? []);
  const linkedOrgIds = new Set(note?.linkedOrgs?.map((o) => o.id) ?? []);

  const searchResults = (postsData?.posts?.posts ?? []).filter(
    (p) => !linkedPostIds.has(p.id)
  );
  const availableOrgs = (orgsData?.organizations ?? []).filter(
    (o) => !linkedOrgIds.has(o.id)
  );

  // ─── Dirty check ──────────────────────────────────────────────
  const isDirty =
    initialized &&
    note != null &&
    (content !== note.content ||
      severity !== note.severity ||
      isPublic !== note.isPublic ||
      (ctaText || "") !== (note.ctaText || ""));

  const mutationContext = { additionalTypenames: ["Note", "NoteConnection"] };

  // ─── Actions ──────────────────────────────────────────────────
  const handleSave = async () => {
    setSaving(true);
    await updateNote(
      { id: noteId, content, severity, isPublic, ctaText: ctaText || null },
      mutationContext
    );
    setSaving(false);
    setInitialized(false);
    reexecuteQuery({ requestPolicy: "network-only" });
  };

  const handleDelete = async () => {
    await deleteNote({ id: noteId }, mutationContext);
    router.push("/admin/notes");
  };

  const handleLink = async (noteableType: string, noteableId: string) => {
    await linkNote(
      { noteId, noteableType, noteableId },
      mutationContext
    );
    reexecuteQuery({ requestPolicy: "network-only" });
  };

  const handleUnlink = async (noteableType: string, noteableId: string) => {
    await unlinkNote(
      { noteId, noteableType, noteableId },
      mutationContext
    );
    reexecuteQuery({ requestPolicy: "network-only" });
  };

  // ─── Loading / error states ───────────────────────────────────

  if (fetching && !note) return <AdminLoader label="Loading note..." />;

  if (error) {
    return (
      <div className="min-h-screen bg-background px-4 py-4">
        <div className="max-w-7xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-danger-text mb-4">Error Loading Note</h1>
          <p className="text-muted-foreground mb-4">{error.message}</p>
          <Link href="/admin/notes" className="text-link hover:text-link-hover">Back to Notes</Link>
        </div>
      </div>
    );
  }

  if (!note) {
    return (
      <div className="min-h-screen bg-background px-4 py-4">
        <div className="max-w-7xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-foreground mb-4">Note Not Found</h1>
          <Link href="/admin/notes" className="text-link hover:text-link-hover">Back to Notes</Link>
        </div>
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="min-h-screen bg-background px-4 py-4">
      <div className="max-w-7xl mx-auto">

        {/* ── Header bar ─────────────────────────────────────────────── */}
        <div className="flex items-center justify-between mb-4">
          <Link
            href="/admin/notes"
            className="inline-flex items-center text-muted-foreground hover:text-foreground text-sm"
          >
            <ArrowLeft className="w-4 h-4 mr-1" /> Back to Notes
          </Link>

          <div className="flex items-center gap-2">
            <Button
              size="sm"
              onClick={handleSave}
              disabled={saving || !isDirty}
            >
              {saving ? "Saving\u2026" : "Save"}
            </Button>

            {note.sourceUrl && (
              <a
                href={note.sourceUrl.startsWith("http") ? note.sourceUrl : `https://${note.sourceUrl}`}
                target="_blank"
                rel="noopener noreferrer"
                className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
                title="View source"
              >
                <ExternalLink className="w-4 h-4" />
              </a>
            )}

            <DropdownMenu>
              <DropdownMenuTrigger render={<Button variant="outline" size="sm" />}>
                {"\u22EF"}
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem variant="destructive" onSelect={() => setShowDeleteDialog(true)}>
                  Delete Note
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* ── Two-column layout ──────────────────────────────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-6">

          {/* ── LEFT COLUMN (60%) ──────────────────────────────────── */}
          <div className="space-y-6">

            {/* Title area — severity + visibility badges */}
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold text-foreground">Note</h1>
              <Badge variant={SEVERITY_BADGE_VARIANT[note.severity] || "secondary"}>
                {note.severity}
              </Badge>
              <Badge variant={note.isPublic ? "success" : "secondary"}>
                {note.isPublic ? "Public" : "Internal"}
              </Badge>
              {note.expiredAt && (
                <Badge variant="secondary">Expired</Badge>
              )}
            </div>

            {/* Content */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Content</SectionLabel>
              <Textarea
                value={content}
                onChange={(e) => setContent(e.target.value)}
                rows={6}
                className="w-full"
              />
            </div>

            {/* Call to Action */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Call to Action</SectionLabel>
              <Input
                value={ctaText}
                onChange={(e) => setCtaText(e.target.value)}
                placeholder="e.g. Learn More, Apply Now"
                className="h-8 text-sm"
              />
            </div>

            {/* Linked Posts */}
            <div className="border-t border-border pt-4">
              <div className="flex justify-between items-center mb-3">
                <SectionLabel>Linked Posts</SectionLabel>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => { setShowLinkPostDialog(true); setPostSearch(""); }}
                >
                  <Plus className="w-3.5 h-3.5 mr-1" />
                  Link Post
                </Button>
              </div>
              {note.linkedPosts && note.linkedPosts.length > 0 ? (
                <div className="space-y-1.5">
                  {note.linkedPosts.map((post) => (
                    <div
                      key={post.id}
                      className="flex items-center justify-between gap-2"
                    >
                      <Link
                        href={`/admin/posts/${post.id}`}
                        className="text-sm text-link hover:text-link-hover truncate"
                      >
                        {post.title}
                      </Link>
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        className="text-muted-foreground hover:text-destructive shrink-0"
                        onClick={() => handleUnlink("post", post.id)}
                        title="Unlink post"
                      >
                        <X className="size-3.5" />
                      </Button>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-text-faint italic">
                  Not linked to any posts
                </p>
              )}
            </div>

            {/* Linked Organizations */}
            <div className="border-t border-border pt-4">
              <div className="flex justify-between items-center mb-3">
                <SectionLabel>Linked Organizations</SectionLabel>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setShowLinkOrgDialog(true)}
                >
                  <Plus className="w-3.5 h-3.5 mr-1" />
                  Link Org
                </Button>
              </div>
              {note.linkedOrgs && note.linkedOrgs.length > 0 ? (
                <div className="space-y-1.5">
                  {note.linkedOrgs.map((org) => (
                    <div
                      key={org.id}
                      className="flex items-center justify-between gap-2"
                    >
                      <Link
                        href={`/admin/organizations/${org.id}`}
                        className="text-sm text-link hover:text-link-hover truncate"
                      >
                        {org.name}
                      </Link>
                      <Button
                        variant="ghost"
                        size="icon-xs"
                        className="text-muted-foreground hover:text-destructive shrink-0"
                        onClick={() => handleUnlink("organization", org.id)}
                        title="Unlink organization"
                      >
                        <X className="size-3.5" />
                      </Button>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-text-faint italic">
                  Not linked to any organizations
                </p>
              )}
            </div>
          </div>

          {/* ── RIGHT COLUMN (40%) ─────────────────────────────────── */}
          <div className="space-y-6">

            {/* Editable properties */}
            <div>
              <SectionLabel>Properties</SectionLabel>
              <div className="space-y-3 text-sm">
                <div className="flex justify-between items-center">
                  <span className="text-muted-foreground">Severity</span>
                  <Select value={severity} onValueChange={(val) => val !== null && setSeverity(val)}>
                    <SelectTrigger className="h-7 w-auto min-w-0 gap-1 text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="urgent">Urgent</SelectItem>
                      <SelectItem value="notice">Notice</SelectItem>
                      <SelectItem value="info">Info</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex justify-between items-center">
                  <Label htmlFor="note-public" className="text-muted-foreground font-normal">
                    Public
                  </Label>
                  <Switch
                    id="note-public"
                    checked={isPublic}
                    onCheckedChange={setIsPublic}
                  />
                </div>
              </div>
            </div>

            {/* Read-only metadata */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Details</SectionLabel>
              <div className="space-y-2 text-sm">
                {note.sourceUrl && (
                  <div className="flex justify-between gap-4">
                    <span className="text-muted-foreground shrink-0">Source</span>
                    <a
                      href={note.sourceUrl.startsWith("http") ? note.sourceUrl : `https://${note.sourceUrl}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-link hover:text-link-hover truncate text-right"
                    >
                      {note.sourceUrl}
                    </a>
                  </div>
                )}

                <div className="flex justify-between">
                  <span className="text-muted-foreground">Created by</span>
                  <span className="text-foreground font-medium">{note.createdBy}</span>
                </div>

                <div className="flex justify-between">
                  <span className="text-muted-foreground">Created</span>
                  <span className="text-foreground">{formatDate(note.createdAt)}</span>
                </div>

                <div className="flex justify-between">
                  <span className="text-muted-foreground">Updated</span>
                  <span className="text-foreground">{formatDate(note.updatedAt)}</span>
                </div>

                {note.expiredAt && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Expired</span>
                    <span className="text-foreground">{formatDate(note.expiredAt)}</span>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* ── Delete confirmation dialog ────────────────────────────── */}
      <Dialog
        open={showDeleteDialog}
        onOpenChange={(open) => !open && setShowDeleteDialog(false)}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Delete Note</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            This will permanently delete this note and unlink it from all
            posts and organizations. This action cannot be undone.
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDeleteDialog(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleDelete}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* ── Link Post dialog ──────────────────────────────────────── */}
      <Dialog
        open={showLinkPostDialog}
        onOpenChange={(open) => { if (!open) setShowLinkPostDialog(false); }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Link Post</DialogTitle>
          </DialogHeader>
          <Input
            value={postSearch}
            onChange={(e) => setPostSearch(e.target.value)}
            placeholder="Search posts by title..."
            className="text-sm"
            autoFocus
          />
          <div className="max-h-64 overflow-y-auto -mx-1">
            {postSearch.length < 2 ? (
              <p className="text-sm text-muted-foreground px-1 py-2">
                Type at least 2 characters to search
              </p>
            ) : postsFetching ? (
              <p className="text-sm text-muted-foreground px-1 py-2">Searching...</p>
            ) : searchResults.length === 0 ? (
              <p className="text-sm text-muted-foreground px-1 py-2">No matching posts</p>
            ) : (
              <div className="space-y-0.5">
                {searchResults.map((post) => (
                  <Button
                    key={post.id}
                    variant="ghost"
                    size="sm"
                    className="w-full justify-start font-normal truncate"
                    onClick={() => handleLink("post", post.id)}
                  >
                    {post.title}
                  </Button>
                ))}
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>

      {/* ── Link Organization dialog ──────────────────────────────── */}
      <Dialog
        open={showLinkOrgDialog}
        onOpenChange={(open) => { if (!open) setShowLinkOrgDialog(false); }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Link Organization</DialogTitle>
          </DialogHeader>
          <div className="max-h-64 overflow-y-auto -mx-1">
            {availableOrgs.length === 0 ? (
              <p className="text-sm text-muted-foreground px-1 py-2">
                {(orgsData?.organizations ?? []).length === 0
                  ? "Loading organizations..."
                  : "All organizations are already linked"}
              </p>
            ) : (
              <div className="space-y-0.5">
                {availableOrgs.map((org) => (
                  <Button
                    key={org.id}
                    variant="ghost"
                    size="sm"
                    className="w-full justify-start font-normal truncate"
                    onClick={() => handleLink("organization", org.id)}
                  >
                    {org.name}
                  </Button>
                ))}
              </div>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
