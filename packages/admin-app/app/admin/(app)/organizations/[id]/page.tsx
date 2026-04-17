"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { useState, useMemo } from "react";
import { useQuery, useMutation } from "urql";
import {
  ArrowLeft,
  Building2,
  MoreHorizontal,
  Trash2,
  User,
} from "lucide-react";

import { AdminLoader } from "@/components/admin/AdminLoader";
import { TagsSection } from "@/components/admin/TagsSection";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
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
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";

import {
  OrganizationDetailFullQuery,
  UpdateOrganizationMutation,
  DeleteOrganizationMutation,
  ApproveOrganizationMutation,
  RejectOrganizationMutation,
  SuspendOrganizationMutation,
  SetOrganizationStatusMutation,
  ToggleChecklistItemMutation,
  AddOrgTagMutation,
  RemoveOrgTagMutation,
} from "@/lib/graphql/organizations";
import { TagKindsQuery, TagsQuery } from "@/lib/graphql/tags";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const orgMutationContext = { additionalTypenames: ["Organization", "Checklist"] };

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleDateString();
}

function statusBadgeVariant(status: string): "success" | "warning" | "danger" | "info" | "secondary" {
  switch (status) {
    case "approved": return "success";
    case "pending_review": return "warning";
    case "rejected": return "danger";
    case "suspended": return "secondary";
    default: return "secondary";
  }
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

export default function OrganizationDetailPage() {
  const params = useParams();
  const router = useRouter();
  const orgId = params.id as string;

  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editLoading, setEditLoading] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [rejectReason, setRejectReason] = useState("");
  const [showRejectDialog, setShowRejectDialog] = useState(false);
  const [showSuspendDialog, setShowSuspendDialog] = useState(false);
  const [suspendReason, setSuspendReason] = useState("");

  // --- Data query ---
  const [{ data: orgData, fetching: orgLoading, error: orgError }] = useQuery({
    query: OrganizationDetailFullQuery,
    variables: { id: orgId },
  });

  const org = orgData?.organization;
  const posts = org?.posts?.posts || [];
  const checklist = org?.checklist;

  // --- Mutations ---
  const [, updateOrg] = useMutation(UpdateOrganizationMutation);
  const [, deleteOrg] = useMutation(DeleteOrganizationMutation);
  const [, approveOrg] = useMutation(ApproveOrganizationMutation);
  const [, rejectOrg] = useMutation(RejectOrganizationMutation);
  const [, suspendOrg] = useMutation(SuspendOrganizationMutation);
  const [, setOrgStatus] = useMutation(SetOrganizationStatusMutation);
  const [, toggleChecklistItem] = useMutation(ToggleChecklistItemMutation);
  const [, addOrgTag] = useMutation(AddOrgTagMutation);
  const [, removeOrgTag] = useMutation(RemoveOrgTagMutation);

  // --- Tag data ---
  const [{ data: kindsData }] = useQuery({ query: TagKindsQuery });
  const [{ data: allTagsData }] = useQuery({ query: TagsQuery });

  const orgTagKinds = useMemo(
    () => (kindsData?.tagKinds || [])
      .filter((k) => k.allowedResourceTypes.includes("organization"))
      .map((k) => ({ slug: k.slug, displayName: k.displayName, locked: k.locked })),
    [kindsData]
  );

  const allTagsByKind = useMemo(() => {
    const map: Record<string, Array<{ id: string; value: string; displayName?: string | null; color?: string | null }>> = {};
    for (const tag of allTagsData?.tags || []) {
      if (!map[tag.kind]) map[tag.kind] = [];
      map[tag.kind].push(tag);
    }
    return map;
  }, [allTagsData]);

  const orgTags = org?.tags || [];

  // --- Tag handlers ---
  const handleAddOrgTags = async (kindSlug: string, newTags: Array<{ value: string; displayName: string }>) => {
    try {
      await Promise.all(
        newTags.map((t) =>
          addOrgTag(
            { organizationId: orgId, tagKind: kindSlug, tagValue: t.value, displayName: t.displayName },
            orgMutationContext,
          )
        )
      );
    } catch (err) {
      console.error("Failed to add tags:", err);
    }
  };

  const handleRemoveOrgTag = async (tagId: string) => {
    if (!orgId) return;
    try {
      await removeOrgTag({ organizationId: orgId, tagId }, orgMutationContext);
    } catch (err) {
      console.error("Failed to remove tag:", err);
    }
  };

  // --- Action helpers ---
  const withAction = (name: string, fn: () => Promise<unknown>) => async () => {
    setActionInProgress(name);
    try {
      await fn();
    } catch (err) {
      console.error(`Failed to ${name}:`, err);
    } finally {
      setActionInProgress(null);
    }
  };

  const startEditing = () => {
    if (!org) return;
    setEditName(org.name);
    setEditDescription(org.description || "");
    setEditing(true);
    setEditError(null);
  };

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editName.trim()) return;
    setEditLoading(true);
    setEditError(null);
    try {
      const result = await updateOrg(
        { id: orgId, name: editName.trim(), description: editDescription.trim() || null },
        orgMutationContext,
      );
      if (result.error) throw result.error;
      setEditing(false);
    } catch (err: any) {
      setEditError(err.message || "Failed to update organization");
    } finally {
      setEditLoading(false);
    }
  };

  const handleDelete = withAction("delete", async () => {
    await deleteOrg({ id: orgId }, orgMutationContext);
    router.push("/admin/organizations");
  });

  const handleApprove = withAction("approve", async () => {
    const result = await approveOrg({ id: orgId }, orgMutationContext);
    if (result.error) throw result.error;
  });

  const handleReject = async () => {
    if (!rejectReason.trim()) return;
    setActionInProgress("reject");
    try {
      const result = await rejectOrg(
        { id: orgId, reason: rejectReason.trim() },
        orgMutationContext,
      );
      if (result.error) throw result.error;
      setShowRejectDialog(false);
      setRejectReason("");
    } catch (err: any) {
      console.error("Failed to reject:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleSuspend = async () => {
    if (!suspendReason.trim()) return;
    setActionInProgress("suspend");
    try {
      const result = await suspendOrg(
        { id: orgId, reason: suspendReason.trim() },
        orgMutationContext,
      );
      if (result.error) throw result.error;
      setShowSuspendDialog(false);
      setSuspendReason("");
    } catch (err: any) {
      console.error("Failed to suspend:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleStatusChange = (newStatus: string) => {
    if (!org || newStatus === org.status) return;
    if (newStatus === "approved") {
      if (!checklist?.allChecked) return;
      handleApprove();
    } else if (newStatus === "rejected") {
      setShowRejectDialog(true);
    } else if (newStatus === "suspended") {
      setShowSuspendDialog(true);
    } else {
      withAction("status", async () => {
        const result = await setOrgStatus({ id: orgId, status: newStatus }, orgMutationContext);
        if (result.error) throw result.error;
      })();
    }
  };

  const handleToggleChecklist = async (key: string, checked: boolean) => {
    try {
      const result = await toggleChecklistItem(
        { organizationId: orgId, checklistKey: key, checked },
        orgMutationContext,
      );
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to toggle checklist item:", err);
    }
  };

  // --- Loading / error states ---
  if (orgLoading) return <AdminLoader label="Loading source..." />;

  if (orgError) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-danger-text mb-4">Error</h1>
          <p className="text-muted-foreground mb-4">{orgError.message}</p>
          <Link href="/admin/organizations" className="text-link hover:text-link-hover">
            Back to Sources
          </Link>
        </div>
      </div>
    );
  }

  if (!org) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-foreground mb-4">Source Not Found</h1>
          <Link href="/admin/organizations" className="text-link hover:text-link-hover">
            Back to Sources
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-background px-4 py-4">
      <div className="max-w-7xl mx-auto">

        {/* ── Header bar ─────────────────────────────────────────────── */}
        <div className="flex items-center justify-between mb-4">
          <Link
            href="/admin/organizations"
            className="inline-flex items-center text-muted-foreground hover:text-foreground text-sm"
          >
            <ArrowLeft className="w-4 h-4 mr-1" /> Back to Sources
          </Link>

          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={startEditing}>
              Edit
            </Button>

            {org.status === "pending_review" && (
              <>
                <Button
                  variant="success"
                  size="sm"
                  onClick={handleApprove}
                  disabled={actionInProgress !== null || !checklist?.allChecked}
                  title={!checklist?.allChecked ? "Complete the pre-launch checklist first" : undefined}
                >
                  {actionInProgress === "approve" ? "..." : "Approve"}
                </Button>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={() => setShowRejectDialog(true)}
                  disabled={actionInProgress !== null}
                >
                  Reject
                </Button>
              </>
            )}

            <Select
              value={org.status}
              disabled={actionInProgress !== null}
              onValueChange={(val) => val !== null && handleStatusChange(val)}
            >
              <SelectTrigger className="h-7 w-auto min-w-0 gap-1 rounded-full px-2.5 text-xs font-medium">
                <Badge variant={statusBadgeVariant(org.status)} className="pointer-events-none">
                  <SelectValue />
                </Badge>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="pending_review">Pending Review</SelectItem>
                <SelectItem value="approved">Approved</SelectItem>
                <SelectItem value="rejected">Rejected</SelectItem>
                <SelectItem value="suspended">Suspended</SelectItem>
              </SelectContent>
            </Select>

            <DropdownMenu>
              <DropdownMenuTrigger render={<Button variant="outline" size="sm" disabled={actionInProgress !== null} />}>
                  <MoreHorizontal className="w-4 h-4" />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {org.status === "approved" && (
                  <>
                    <DropdownMenuItem onSelect={() => handleStatusChange("suspended")}>
                      Suspend
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                  </>
                )}
                <DropdownMenuItem variant="destructive" onSelect={handleDelete} disabled={actionInProgress !== null}>
                  <Trash2 className="w-4 h-4" />
                  Delete {org.sourceType === "individual" ? "Individual" : "Organization"}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* ── Two-column layout ──────────────────────────────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-[2fr_1fr] gap-6">

          {/* ── LEFT COLUMN (60%) ──────────────────────────────────── */}
          <div className="space-y-6">

            {/* Title / edit form */}
            {editing ? (
              <form onSubmit={handleUpdate} className="space-y-3">
                <Input
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  className="text-lg font-bold"
                  autoFocus
                  disabled={editLoading}
                />
                <Textarea
                  value={editDescription}
                  onChange={(e) => setEditDescription(e.target.value)}
                  placeholder="Description (optional)"
                  rows={3}
                  disabled={editLoading}
                />
                <div className="flex items-center gap-2">
                  <Button type="submit" disabled={editLoading || !editName.trim()} loading={editLoading}>
                    Save
                  </Button>
                  <Button type="button" variant="ghost" onClick={() => setEditing(false)}>
                    Cancel
                  </Button>
                  {editError && <span className="text-danger-text text-sm">{editError}</span>}
                </div>
              </form>
            ) : (
              <>
                <div className="flex items-center gap-2.5">
                  <h1 className="text-2xl font-bold text-foreground">{org.name}</h1>
                  <Badge variant="outline" className="text-[11px]">
                    {org.sourceType === "individual" ? (
                      <><User className="h-3 w-3" /> Individual</>
                    ) : (
                      <><Building2 className="h-3 w-3" /> Organization</>
                    )}
                  </Badge>
                </div>
                {org.description && (
                  <p className="text-muted-foreground -mt-3">{org.description}</p>
                )}
              </>
            )}

            {/* Posts */}
            <PostsSection posts={posts} />
          </div>

          {/* ── RIGHT COLUMN (40%) ─────────────────────────────────── */}
          <div className="space-y-6">

            {/* Metadata */}
            <div>
              <SectionLabel>Details</SectionLabel>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Type</span>
                  <Badge variant="outline" className="text-[11px]">
                    {org.sourceType === "individual" ? (
                      <><User className="h-3 w-3" /> Individual</>
                    ) : (
                      <><Building2 className="h-3 w-3" /> Organization</>
                    )}
                  </Badge>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Status</span>
                  <Badge variant={statusBadgeVariant(org.status)}>
                    {org.status.replace(/_/g, " ")}
                  </Badge>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Created</span>
                  <span className="text-foreground">{formatDate(org.createdAt)}</span>
                </div>
                {org.updatedAt && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Last updated</span>
                    <span className="text-foreground">{formatDate(org.updatedAt)}</span>
                  </div>
                )}
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Posts</span>
                  <span className="text-foreground font-medium">{posts.length}</span>
                </div>
              </div>
            </div>

            {/* Tags */}
            <div className="border-t border-border pt-4">
              <TagsSection
                tags={orgTags}
                applicableKinds={orgTagKinds}
                allTagsByKind={allTagsByKind}
                onRemoveTag={handleRemoveOrgTag}
                onAddTags={handleAddOrgTags}
              />
            </div>

            {/* Pre-Launch Checklist */}
            {org.status === "pending_review" && checklist && (
              <div className="border-t border-border pt-4">
                <SectionLabel>Pre-Launch Checklist</SectionLabel>
                <div className="space-y-2">
                  {checklist.items.map((item) => (
                    <label
                      key={item.key}
                      className="flex items-center gap-3 cursor-pointer group"
                    >
                      <input
                        type="checkbox"
                        checked={item.checked}
                        onChange={(e) => handleToggleChecklist(item.key, e.target.checked)}
                        className="h-4 w-4 rounded border-border text-primary focus:ring-primary/50 cursor-pointer"
                      />
                      <span className={`text-sm ${item.checked ? "text-muted-foreground line-through" : "text-foreground"}`}>
                        {item.label}
                      </span>
                      {item.checked && item.checkedAt && (
                        <span className="text-xs text-muted-foreground">
                          {formatDate(item.checkedAt)}
                        </span>
                      )}
                    </label>
                  ))}
                </div>
                {!checklist.allChecked && (
                  <p className="text-xs text-warning-text mt-3">
                    Complete all items before approving this source.
                  </p>
                )}
              </div>
            )}

          </div>
        </div>
      </div>

      {/* ── Reject Dialog ─────────────────────────────────────── */}
      <Dialog open={showRejectDialog} onOpenChange={setShowRejectDialog}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Reject {org.sourceType === "individual" ? "Individual" : "Organization"}</DialogTitle>
          </DialogHeader>
          <Textarea
            value={rejectReason}
            onChange={(e) => setRejectReason(e.target.value)}
            placeholder="Reason for rejection..."
            rows={3}
            autoFocus
          />
          <DialogFooter>
            <Button variant="ghost" onClick={() => { setShowRejectDialog(false); setRejectReason(""); }}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleReject}
              disabled={!rejectReason.trim() || actionInProgress !== null}
              loading={actionInProgress === "reject"}
            >
              Reject
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* ── Suspend Dialog ────────────────────────────────────── */}
      <Dialog open={showSuspendDialog} onOpenChange={setShowSuspendDialog}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Suspend {org.sourceType === "individual" ? "Individual" : "Organization"}</DialogTitle>
          </DialogHeader>
          <Textarea
            value={suspendReason}
            onChange={(e) => setSuspendReason(e.target.value)}
            placeholder="Reason for suspension..."
            rows={3}
            autoFocus
          />
          <DialogFooter>
            <Button variant="ghost" onClick={() => { setShowSuspendDialog(false); setSuspendReason(""); }}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleSuspend}
              disabled={!suspendReason.trim() || actionInProgress !== null}
              loading={actionInProgress === "suspend"}
            >
              Suspend
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

    </div>
  );
}

// ---------------------------------------------------------------------------
// Posts section
// ---------------------------------------------------------------------------

type PostStatusTab = "active" | "draft" | "rejected";

const POST_STATUS_TABS: { value: PostStatusTab; label: string }[] = [
  { value: "active", label: "Active" },
  { value: "draft", label: "Drafts" },
  { value: "rejected", label: "Rejected" },
];

type PostData = {
  id: string;
  title: string;
  status: string;
  postType?: string | null;
  createdAt: string;
};

function postTypeBadgeVariant(postType?: string | null): "info" | "success" | "warning" | "spotlight" | "secondary" | "danger" {
  // 9-type enum from migration 216.
  switch (postType) {
    case "story": return "info";
    case "update": return "secondary";
    case "action": return "warning";
    case "event": return "warning";
    case "need": return "danger";
    case "aid": return "success";
    case "person": return "spotlight";
    case "business": return "info";
    case "reference": return "info";
    default: return "secondary";
  }
}

function PostsSection({ posts }: { posts: PostData[] }) {
  const [tab, setTab] = useState<PostStatusTab>("active");

  const counts = {
    active: posts.filter((p) => p.status === "active").length,
    draft: posts.filter((p) => p.status === "draft").length,
    rejected: posts.filter((p) => p.status === "rejected").length,
  };

  const filtered = posts.filter((p) => p.status === tab);

  return (
    <div className="border-t border-border pt-4">
      <div className="flex items-center justify-between mb-3">
        <SectionLabel>
          Posts {posts.length > 0 && <span className="text-muted-foreground font-normal">({posts.length})</span>}
        </SectionLabel>
      </div>
      <Tabs value={tab} onValueChange={(v) => setTab(v as PostStatusTab)}>
        <TabsList>
          {POST_STATUS_TABS.map((t) => (
            <TabsTrigger key={t.value} value={t.value}>
              {t.label}
              {counts[t.value] > 0 && (
                <span className="text-xs opacity-60 tabular-nums">{counts[t.value]}</span>
              )}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      {filtered.length === 0 ? (
        <p className="text-muted-foreground text-sm">No {tab} posts.</p>
      ) : (
        <div className="space-y-2">
          {filtered.map((post) => (
            <PostRow key={post.id} post={post} />
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Post row
// ---------------------------------------------------------------------------

function postStatusBadgeVariant(status: string): "success" | "info" | "danger" | "secondary" {
  switch (status) {
    case "active": return "success";
    case "draft": return "info";
    case "rejected": return "danger";
    default: return "secondary";
  }
}

function PostRow({ post }: { post: PostData }) {
  const status = post.status;

  return (
    <div className="flex items-center gap-3 p-3 rounded-lg border border-border bg-card hover:bg-accent/30 transition-colors min-w-0">
      <Link href={`/admin/posts/${post.id}`} className="font-medium text-foreground truncate hover:underline min-w-0 shrink">
        {post.title}
      </Link>
      <div className="flex items-center gap-1.5 shrink-0 ml-auto">
        <Badge variant={postStatusBadgeVariant(status)}>{status}</Badge>
        <Badge variant={postTypeBadgeVariant(post.postType)}>{post.postType || "unknown"}</Badge>
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {formatDate(post.createdAt)}
        </span>
      </div>
    </div>
  );
}

