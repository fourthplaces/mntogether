"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { useQuery, useMutation } from "urql";
import { useState } from "react";
import {
  PostDetailFullQuery,
  ApprovePostMutation,
  RejectPostMutation,
  ArchivePostMutation,
  DeletePostMutation,
  ReactivatePostMutation,
  AddPostTagMutation,
  RemovePostTagMutation,
  RegeneratePostMutation,
  RegeneratePostTagsMutation,
} from "@/lib/graphql/posts";
import { TagKindsQuery, TagsQuery } from "@/lib/graphql/tags";
import { markdownComponents } from "@/lib/markdown-components";

const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

function formatTime12h(time24: string): string {
  const [h, m] = time24.split(":").map(Number);
  const suffix = h >= 12 ? "PM" : "AM";
  const h12 = h % 12 || 12;
  return `${h12}:${m.toString().padStart(2, "0")} ${suffix}`;
}

interface ScheduleItem {
  id: string;
  dayOfWeek?: number | null;
  opensAt?: string | null;
  closesAt?: string | null;
  timezone: string;
  notes?: string | null;
  rrule?: string | null;
  dtstart?: string | null;
  dtend?: string | null;
  isAllDay: boolean;
  durationMinutes?: number | null;
}

function isScheduleExpired(s: ScheduleItem): boolean {
  if (s.dtend && !s.rrule) return new Date(s.dtend) < new Date();
  if (s.dtstart && !s.rrule && !s.dtend) return new Date(s.dtstart) < new Date();
  return false;
}

function formatSchedule(s: ScheduleItem): string {
  if (s.dtstart && s.dayOfWeek == null) {
    const date = new Date(s.dtstart);
    const dateStr = date.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
    const timeStr = s.opensAt && s.closesAt
      ? `${formatTime12h(s.opensAt)} – ${formatTime12h(s.closesAt)}`
      : s.opensAt ? formatTime12h(s.opensAt) : "";
    return [dateStr, timeStr].filter(Boolean).join("  ");
  }

  const dayName = s.dayOfWeek != null ? DAY_NAMES[s.dayOfWeek] : "";
  const timeStr = s.opensAt && s.closesAt
    ? `${formatTime12h(s.opensAt)} – ${formatTime12h(s.closesAt)}`
    : s.opensAt ? formatTime12h(s.opensAt) : "";

  let suffix = "";
  if (s.rrule?.includes("INTERVAL=2")) suffix = " (every other week)";
  if (s.rrule?.includes("FREQ=MONTHLY")) suffix = " (monthly)";

  return [dayName, timeStr, suffix].filter(Boolean).join("  ");
}

export default function PostDetailPage() {
  const params = useParams();
  const router = useRouter();
  const postId = params.id as string;
  const [isUpdating, setIsUpdating] = useState(false);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [showTagModal, setShowTagModal] = useState(false);
  const [selectedKind, setSelectedKind] = useState("");
  const [tagValue, setTagValue] = useState("");
  const [tagDisplayName, setTagDisplayName] = useState("");
  const [isCreatingNewTag, setIsCreatingNewTag] = useState(false);

  // GraphQL: fetch post detail + notes in single query
  const [{ data: postData, fetching: isLoading, error }] = useQuery({
    query: PostDetailFullQuery,
    variables: { id: postId },
  });
  const post = postData?.post;
  const notes = postData?.entityNotes || [];

  // GraphQL mutations
  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, rejectPost] = useMutation(RejectPostMutation);
  const [, archivePost] = useMutation(ArchivePostMutation);
  const [, deletePost] = useMutation(DeletePostMutation);
  const [, reactivatePost] = useMutation(ReactivatePostMutation);
  const [, addPostTag] = useMutation(AddPostTagMutation);
  const [, removePostTag] = useMutation(RemovePostTagMutation);
  const [, regeneratePost] = useMutation(RegeneratePostMutation);
  const [, regeneratePostTags] = useMutation(RegeneratePostTagsMutation);

  // Tag modal: load kinds and tags
  const [{ data: kindsData }] = useQuery({
    query: TagKindsQuery,
    pause: !showTagModal,
  });
  const [{ data: kindTagsData }] = useQuery({
    query: TagsQuery,
    pause: !showTagModal || !selectedKind,
  });

  const availableKinds = kindsData?.tagKinds || [];
  const availableTags = (kindTagsData?.tags || []).filter(
    (t) => t.kind === selectedKind
  );

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  const statusBadgeVariant = (status: string): "success" | "warning" | "danger" | "info" | "secondary" => {
    switch (status) {
      case "active": return "success";
      case "pending_approval": return "warning";
      case "rejected": return "danger";
      case "draft": return "info";
      case "archived": return "secondary";
      default: return "secondary";
    }
  };

  const tags = post?.tags || [];

  // Group tags by kind for display
  const tagsByKind: Record<string, typeof tags> = {};
  for (const tag of tags) {
    if (!tagsByKind[tag.kind]) tagsByKind[tag.kind] = [];
    tagsByKind[tag.kind].push(tag);
  }

  const mutationContext = { additionalTypenames: ["Post", "PostConnection", "PostStats"] };

  const handleAddTag = async () => {
    if (!postId || !selectedKind || !tagValue) return;
    setIsUpdating(true);
    try {
      await addPostTag({
        postId,
        tagKind: selectedKind,
        tagValue: tagValue,
        displayName: tagDisplayName || tagValue,
      }, mutationContext);
      setTagValue("");
      setTagDisplayName("");
    } catch (err) {
      console.error("Failed to add tag:", err);
    } finally {
      setIsUpdating(false);
    }
  };

  const handleRemoveTag = async (tagId: string) => {
    if (!postId) return;
    setIsUpdating(true);
    try {
      await removePostTag({ postId, tagId }, mutationContext);
    } catch (err) {
      console.error("Failed to remove tag:", err);
    } finally {
      setIsUpdating(false);
    }
  };

  const handleRegenerate = async () => {
    setActionInProgress("regenerate");
    try {
      await regeneratePost({ id: postId }, mutationContext);
    } catch (err) {
      console.error("Failed to regenerate post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleRegenerateTags = async () => {
    setActionInProgress("regenerate_tags");
    try {
      await regeneratePostTags({ id: postId }, mutationContext);
    } catch (err) {
      console.error("Failed to regenerate tags:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleArchive = async () => {
    setActionInProgress("archive");
    try {
      await archivePost({ id: postId }, mutationContext);
    } catch (err) {
      console.error("Failed to archive post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleDelete = async () => {
    setActionInProgress("delete");
    try {
      await deletePost({ id: postId }, mutationContext);
      router.push("/admin/posts");
    } catch (err) {
      console.error("Failed to delete post:", err);
      setActionInProgress(null);
    }
  };

  const handleReactivate = async () => {
    setActionInProgress("reactivate");
    try {
      await reactivatePost({ id: postId }, mutationContext);
    } catch (err) {
      console.error("Failed to reactivate post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleApprove = async () => {
    setActionInProgress("approve");
    try {
      await approvePost({ id: postId }, mutationContext);
    } catch (err) {
      console.error("Failed to approve post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    setActionInProgress("reject");
    try {
      await rejectPost({ id: postId, reason: "Rejected by admin" }, mutationContext);
    } catch (err) {
      console.error("Failed to reject post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  if (isLoading) {
    return <AdminLoader label="Loading post..." />;
  }

  if (error) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-danger-text mb-4">Error Loading Post</h1>
            <p className="text-muted-foreground mb-4">{error.message}</p>
            <Link href="/admin/posts" className="text-link hover:text-link-hover">
              Back to Posts
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-foreground mb-4">Post Not Found</h1>
            <Link href="/admin/posts" className="text-link hover:text-link-hover">
              Back to Posts
            </Link>
          </div>
        </div>
      </div>
    );
  }

  const missingFields: string[] = [];
  if (!post.sourceUrl) missingFields.push("source URL");
  if (!post.location) missingFields.push("location");
  if (tags.length === 0) missingFields.push("tags");
  if (!post.contacts || post.contacts.length === 0) missingFields.push("contact info");

  return (
    <div className="min-h-screen bg-background p-6">
      <div className="max-w-4xl mx-auto">
        {/* Back Button */}
        <Link
          href="/admin/posts"
          className="inline-flex items-center text-muted-foreground hover:text-foreground mb-6"
        >
          {"\u2190"} Back to Posts
        </Link>

        {/* Post Header */}
        <div className="bg-card rounded-lg shadow-card p-6 mb-6">
          <div className="flex justify-between items-start mb-4">
            <div className="flex-1">
              <h1 className="text-2xl font-bold text-foreground mb-2">{post.title}</h1>
            </div>
            <div className="flex items-center gap-2">
              <Button asChild variant="outline" size="sm">
                <Link href={`/admin/posts/${postId}/edit`}>Edit</Link>
              </Button>
              <Select
                value={post.status}
                disabled={actionInProgress !== null}
                onValueChange={(newStatus) => {
                  if (newStatus === post.status) return;
                  if (newStatus === "active") handleApprove();
                  else if (newStatus === "rejected") handleReject();
                  else if (newStatus === "archived") handleArchive();
                  else if (newStatus === "pending_approval") handleReactivate();
                }}
              >
                <SelectTrigger className="h-7 w-auto min-w-0 gap-1 rounded-full px-2.5 text-xs font-medium">
                  <Badge variant={statusBadgeVariant(post.status)} className="pointer-events-none">
                    <SelectValue />
                  </Badge>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="draft">Draft</SelectItem>
                  <SelectItem value="pending_approval">Pending</SelectItem>
                  <SelectItem value="active">Active</SelectItem>
                  <SelectItem value="rejected">Rejected</SelectItem>
                  <SelectItem value="archived">Archived</SelectItem>
                </SelectContent>
              </Select>

              {post.status === "active" && (
                <Link
                  href={`/posts/${postId}`}
                  className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
                  title="View public page"
                >
                  {"\u2197"}
                </Link>
              )}

              {post.sourceUrl && (
                <a
                  href={post.sourceUrl.startsWith("http") ? post.sourceUrl : `https://${post.sourceUrl}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
                  title="View source page"
                >
                  {"\u{1F517}"}
                </a>
              )}

              {/* More Actions — Radix DropdownMenu */}
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="sm" disabled={actionInProgress !== null}>
                    {actionInProgress ? "..." : "\u22EF"}
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onSelect={() => setShowTagModal(true)}>
                    Edit Tags
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onSelect={handleRegenerateTags}
                    disabled={actionInProgress !== null}
                  >
                    {actionInProgress === "regenerate_tags" ? "Regenerating..." : "Regenerate Tags"}
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onSelect={handleRegenerate}
                    disabled={actionInProgress !== null}
                  >
                    {actionInProgress === "regenerate" ? "Re-running..." : "Re-run Investigation"}
                  </DropdownMenuItem>
                  {post.status === "active" && (
                    <DropdownMenuItem
                      onSelect={handleArchive}
                      disabled={actionInProgress !== null}
                    >
                      {actionInProgress === "archive" ? "Archiving..." : "Archive (Delist)"}
                    </DropdownMenuItem>
                  )}
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    variant="destructive"
                    onSelect={handleDelete}
                    disabled={actionInProgress !== null}
                  >
                    Delete Post
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>

          {/* Missing Fields Warning */}
          {missingFields.length > 0 && (
            <Alert variant="warning" className="mb-4">
              <AlertDescription>
                <span className="font-medium">Missing fields: </span>
                {missingFields.join(", ")}
              </AlertDescription>
            </Alert>
          )}

          {/* Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-border">
            <div>
              <span className="text-xs text-muted-foreground uppercase">Type</span>
              <p className="text-sm font-medium text-foreground">{post.postType}</p>
            </div>
            <div>
              <span className="text-xs text-muted-foreground uppercase">Category</span>
              <p className="text-sm font-medium text-foreground">{post.category}</p>
            </div>
            {post.urgency && (
              <div>
                <span className="text-xs text-muted-foreground uppercase">Urgency</span>
                <p className="text-sm font-medium text-foreground">{post.urgency}</p>
              </div>
            )}
            {post.location && (
              <div>
                <span className="text-xs text-muted-foreground uppercase">Location</span>
                <p className="text-sm font-medium text-foreground">{post.location}</p>
              </div>
            )}
            <div>
              <span className="text-xs text-muted-foreground uppercase">{post.publishedAt ? "Published" : "Created"}</span>
              <p className="text-sm font-medium text-foreground">{formatDate(post.publishedAt || post.createdAt)}</p>
            </div>
            {post.sourceUrl && (
              <div className="col-span-2">
                <span className="text-xs text-muted-foreground uppercase">Source URL</span>
                <p className="text-sm font-medium truncate">
                  <a
                    href={post.sourceUrl.startsWith("http") ? post.sourceUrl : `https://${post.sourceUrl}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-link hover:text-link-hover"
                  >
                    {post.sourceUrl}
                  </a>
                </p>
              </div>
            )}
            <div>
              <span className="text-xs text-muted-foreground uppercase">Organization</span>
              <p className="text-sm font-medium text-foreground">
                {post.organizationId ? (
                  <Link href={`/admin/organizations/${post.organizationId}`} className="text-admin-accent hover:text-admin-accent-hover">
                    {post.organizationName}
                  </Link>
                ) : (
                  <span className="text-text-faint">None</span>
                )}
              </p>
            </div>
            <div>
              <span className="text-xs text-muted-foreground uppercase">Submitted By</span>
              <p className="text-sm font-medium text-foreground">
                {post.submittedBy?.submitterType === "agent" && post.submittedBy.agentId ? (
                  <Link href={`/admin/agents/${post.submittedBy.agentId}`} className="text-link hover:text-link-hover">
                    {post.submittedBy.agentName || "Agent"} (AI)
                  </Link>
                ) : post.submittedBy?.submitterType === "member" ? (
                  <span>Member</span>
                ) : (
                  <span className="text-text-faint">Unknown</span>
                )}
              </p>
            </div>
          </div>
        </div>

        {/* Contact Info */}
        {post.contacts && post.contacts.length > 0 && (
          <div className="bg-card rounded-lg shadow-card p-6 mb-6">
            <h2 className="text-lg font-semibold text-foreground mb-4">Contact Info</h2>
            <div className="space-y-2">
              {post.contacts.map((c) => (
                <div key={c.id} className="flex items-start gap-3">
                  <span className="text-xs text-muted-foreground uppercase w-20 flex-shrink-0 pt-0.5">{c.contactType}</span>
                  <span className="text-sm text-text-body">
                    {c.contactType === "email" ? (
                      <a href={`mailto:${c.contactValue}`} className="text-link hover:text-link-hover">{c.contactValue}</a>
                    ) : c.contactType === "phone" ? (
                      <a href={`tel:${c.contactValue}`} className="text-link hover:text-link-hover">{c.contactValue}</a>
                    ) : c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social" || c.contactType === "intake_form_url" ? (
                      <a href={c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`} target="_blank" rel="noopener noreferrer" className="text-link hover:text-link-hover break-all">{c.contactValue}</a>
                    ) : (
                      <span>{c.contactValue}</span>
                    )}
                    {c.contactLabel && <span className="text-text-faint ml-2">({c.contactLabel})</span>}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Tags */}
        <div id="tags-section" className="bg-card rounded-lg shadow-card p-6 mb-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-lg font-semibold text-foreground">Tags</h2>
            <Button variant="link" size="sm" onClick={() => setShowTagModal(true)}>
              Edit
            </Button>
          </div>

          {tags.length > 0 ? (
            <div className="space-y-3">
              {Object.entries(tagsByKind).map(([kind, kindTags]) => (
                <div key={kind}>
                  <span className="text-xs text-muted-foreground uppercase">{kind.replace(/_/g, " ")}</span>
                  <div className="flex flex-wrap gap-2 mt-1">
                    {kindTags.map((tag) => (
                      <Badge key={tag.id} variant="secondary" color={tag.color || undefined}>
                        {tag.value}
                      </Badge>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <span className="text-text-faint text-sm">No tags</span>
          )}
        </div>

        {/* Schedule */}
        {post.schedules && post.schedules.length > 0 && (() => {
          const oneOffSchedules = post.schedules!.filter((s) => !s.rrule);
          const allOneOffsExpired = oneOffSchedules.length > 0 && oneOffSchedules.every(isScheduleExpired);
          return (
            <div className="bg-card rounded-lg shadow-card p-6 mb-6">
              <h2 className="text-lg font-semibold text-foreground mb-4">Schedule</h2>
              {allOneOffsExpired && (
                <Alert variant="warning" className="mb-3">
                  <AlertDescription className="text-xs font-medium">This event has passed</AlertDescription>
                </Alert>
              )}
              <div className="space-y-2">
                {post.schedules!.map((s) => (
                  <div key={s.id} className={`flex items-start gap-2 text-text-body ${isScheduleExpired(s) ? "opacity-60" : ""}`}>
                    <svg className="w-4 h-4 mt-0.5 flex-shrink-0 text-text-faint" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    <span className="text-sm">{formatSchedule(s)}</span>
                  </div>
                ))}
              </div>
            </div>
          );
        })()}

        {/* Description */}
        <div className="bg-card rounded-lg shadow-card p-6 mb-6">
          <h2 className="text-lg font-semibold text-foreground mb-4">Description</h2>
          <div className="prose prose-stone max-w-none">
            <ReactMarkdown components={markdownComponents}>
              {post.descriptionMarkdown || post.description || ""}
            </ReactMarkdown>
          </div>
        </div>

        {/* Notes */}
        {notes.length > 0 && (
          <div className="bg-card rounded-lg shadow-card p-6">
            <h2 className="text-lg font-semibold text-foreground mb-4">
              Notes ({notes.length})
            </h2>
            <div className="space-y-2">
              {notes.map((note) => {
                const isExpired = !!note.expiredAt;
                const severityVariant: "danger" | "warning" | "info" =
                  note.severity === "urgent" ? "danger" :
                  note.severity === "notice" ? "warning" : "info";

                return (
                  <div
                    key={note.id}
                    className={`p-3 rounded-lg border ${
                      isExpired ? "border-border bg-secondary opacity-60" : "border-border"
                    }`}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <Badge variant={severityVariant}>{note.severity}</Badge>
                      {note.isPublic && <Badge variant="success">public</Badge>}
                      {isExpired && <Badge variant="secondary">expired</Badge>}
                      <span className="text-xs text-text-faint">
                        {note.createdBy} &middot; {new Date(note.createdAt).toLocaleDateString()}
                      </span>
                    </div>
                    <p className="text-sm text-text-body">{note.content}</p>
                    {note.sourceUrl && (
                      <a
                        href={note.sourceUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-xs text-link hover:text-link-hover mt-1 inline-block"
                      >
                        Source {"\u2197"}
                      </a>
                    )}
                    {note.linkedPosts && note.linkedPosts.filter(p => p.id !== postId).length > 0 && (
                      <div className="flex flex-wrap items-center gap-1 mt-1.5">
                        <span className="text-xs text-text-faint">Also on:</span>
                        {note.linkedPosts.filter(p => p.id !== postId).map((p) => (
                          <Link
                            key={p.id}
                            href={`/admin/posts/${p.id}`}
                            className="text-xs px-1.5 py-0.5 bg-secondary text-secondary-foreground rounded hover:bg-accent hover:text-accent-foreground transition-colors truncate max-w-[200px]"
                            title={p.title}
                          >
                            {p.title}
                          </Link>
                        ))}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {/* Tag Editor Modal — Radix Dialog */}
      <Dialog open={showTagModal} onOpenChange={setShowTagModal}>
        <DialogContent className="max-w-lg max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Edit Tags</DialogTitle>
          </DialogHeader>

          {/* Current tags grouped by kind */}
          {tags.length > 0 ? (
            <div className="space-y-3">
              {Object.entries(tagsByKind).map(([kind, kindTags]) => (
                <div key={kind}>
                  <span className="text-xs text-muted-foreground uppercase font-medium">{kind.replace(/_/g, " ")}</span>
                  <div className="flex flex-wrap gap-2 mt-1">
                    {kindTags.map((tag) => (
                      <Badge key={tag.id} variant="secondary" color={tag.color || undefined} className="gap-1">
                        {tag.value}
                        <button
                          onClick={() => handleRemoveTag(tag.id)}
                          disabled={isUpdating}
                          className="hover:text-destructive ml-1 disabled:opacity-50"
                        >
                          &times;
                        </button>
                      </Badge>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-muted-foreground text-sm">No tags yet.</p>
          )}

          {/* Add tag form — native <select> retained for "Create new..." toggle */}
          <div className="border-t border-border pt-4">
            <h4 className="text-sm font-medium text-foreground mb-3">Add a tag</h4>
            <div className="space-y-3">
              <div>
                <label className="block text-xs text-muted-foreground mb-1">Kind</label>
                <select
                  value={selectedKind}
                  onChange={(e) => {
                    setSelectedKind(e.target.value);
                    setTagValue("");
                    setTagDisplayName("");
                    setIsCreatingNewTag(false);
                  }}
                  className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
                >
                  <option value="">Select a kind...</option>
                  {availableKinds.map((kind) => (
                    <option key={kind.id} value={kind.slug}>
                      {kind.displayName}
                    </option>
                  ))}
                </select>
              </div>

              {selectedKind && (
                <>
                  <div>
                    <label className="block text-xs text-muted-foreground mb-1">Value</label>
                    {isCreatingNewTag ? (
                      <div className="space-y-2">
                        <Input
                          value={tagValue}
                          onChange={(e) => setTagValue(e.target.value)}
                          placeholder="New tag value..."
                          autoFocus
                        />
                        <div>
                          <label className="block text-xs text-muted-foreground mb-1">Display Name</label>
                          <Input
                            value={tagDisplayName}
                            onChange={(e) => setTagDisplayName(e.target.value)}
                            placeholder="Human-readable name..."
                          />
                        </div>
                        <Button
                          variant="link"
                          size="xs"
                          onClick={() => {
                            setIsCreatingNewTag(false);
                            setTagValue("");
                            setTagDisplayName("");
                          }}
                        >
                          Back to list
                        </Button>
                      </div>
                    ) : (
                      <div className="space-y-2">
                        <select
                          value={tagValue}
                          onChange={(e) => {
                            const val = e.target.value;
                            if (val === "__new__") {
                              setIsCreatingNewTag(true);
                              setTagValue("");
                              setTagDisplayName("");
                              return;
                            }
                            setTagValue(val);
                            const match = availableTags.find((t) => t.value === val);
                            setTagDisplayName(match?.displayName || val);
                          }}
                          className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-xs focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
                        >
                          <option value="">Select a value...</option>
                          {availableTags.map((tag) => (
                            <option key={tag.id} value={tag.value}>
                              {tag.value}
                            </option>
                          ))}
                          <option value="__new__">+ Create new...</option>
                        </select>
                      </div>
                    )}
                  </div>

                  <Button
                    onClick={handleAddTag}
                    disabled={isUpdating || !tagValue}
                    loading={isUpdating}
                    className="w-full"
                  >
                    Add Tag
                  </Button>
                </>
              )}
            </div>
          </div>
        </DialogContent>
      </Dialog>

    </div>
  );
}
