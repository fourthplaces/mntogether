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
import { useState, useCallback, useRef } from "react";
import {
  PostDetailFullQuery,
  UpdatePostMutation,
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
import { OrganizationsListQuery } from "@/lib/graphql/organizations";
import { TagKindsQuery, TagsQuery } from "@/lib/graphql/tags";
import { markdownComponents } from "@/lib/markdown-components";
import { POST_TYPES, WEIGHTS } from "@/lib/post-form-constants";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
      ? `${formatTime12h(s.opensAt)} \u2013 ${formatTime12h(s.closesAt)}`
      : s.opensAt ? formatTime12h(s.opensAt) : "";
    return [dateStr, timeStr].filter(Boolean).join("  ");
  }
  const dayName = s.dayOfWeek != null ? DAY_NAMES[s.dayOfWeek] : "";
  const timeStr = s.opensAt && s.closesAt
    ? `${formatTime12h(s.opensAt)} \u2013 ${formatTime12h(s.closesAt)}`
    : s.opensAt ? formatTime12h(s.opensAt) : "";
  let suffix = "";
  if (s.rrule?.includes("INTERVAL=2")) suffix = " (every other week)";
  if (s.rrule?.includes("FREQ=MONTHLY")) suffix = " (monthly)";
  return [dayName, timeStr, suffix].filter(Boolean).join("  ");
}

function formatDate(dateString: string) {
  return new Date(dateString).toLocaleString();
}

function statusBadgeVariant(status: string): "success" | "warning" | "danger" | "info" | "secondary" {
  switch (status) {
    case "active": return "success";
    case "pending_approval": return "warning";
    case "rejected": return "danger";
    case "draft": return "info";
    case "archived": return "secondary";
    default: return "secondary";
  }
}

// ---------------------------------------------------------------------------
// Section label component
// ---------------------------------------------------------------------------

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-3">
      {children}
    </h3>
  );
}

// ---------------------------------------------------------------------------
// Body text preview (read-only, from Root Signal)
// ---------------------------------------------------------------------------

function BodyPreview({ label, text }: { label: string; text?: string | null }) {
  return (
    <div className="border-t border-border pt-4">
      <SectionLabel>{label}</SectionLabel>
      {text ? (
        <p className="text-sm text-text-body leading-relaxed whitespace-pre-wrap">{text}</p>
      ) : (
        <p className="text-sm text-text-faint italic">Not yet generated</p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Inline field — text input that saves on blur
// ---------------------------------------------------------------------------

function InlineTextField({
  label,
  value,
  placeholder,
  onSave,
  className,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onSave: (v: string) => void;
  className?: string;
}) {
  const [localValue, setLocalValue] = useState(value);
  const prevValue = useRef(value);

  // Sync if external value changes (e.g. after mutation response)
  if (value !== prevValue.current) {
    prevValue.current = value;
    setLocalValue(value);
  }

  const handleBlur = () => {
    const trimmed = localValue.trim();
    if (trimmed !== value) {
      onSave(trimmed);
    }
  };

  return (
    <div className={className}>
      <label className="block text-xs text-muted-foreground uppercase mb-1">{label}</label>
      <Input
        value={localValue}
        onChange={(e) => setLocalValue(e.target.value)}
        onBlur={handleBlur}
        placeholder={placeholder}
        className="h-8 text-sm"
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function PostDetailPage() {
  const params = useParams();
  const router = useRouter();
  const postId = params.id as string;
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const [showTagModal, setShowTagModal] = useState(false);
  const [selectedKind, setSelectedKind] = useState("");
  const [tagValue, setTagValue] = useState("");
  const [tagDisplayName, setTagDisplayName] = useState("");
  const [isCreatingNewTag, setIsCreatingNewTag] = useState(false);

  // GraphQL: fetch post detail + notes
  const [{ data: postData, fetching: isLoading, error }] = useQuery({
    query: PostDetailFullQuery,
    variables: { id: postId },
  });
  const post = postData?.post;
  const notes = postData?.entityNotes || [];

  // Organizations for inline dropdown
  const [{ data: orgsData }] = useQuery({ query: OrganizationsListQuery });
  const organizations = orgsData?.organizations ?? [];

  // GraphQL mutations
  const [, updatePost] = useMutation(UpdatePostMutation);
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
  const [{ data: kindsData }] = useQuery({ query: TagKindsQuery, pause: !showTagModal });
  const [{ data: kindTagsData }] = useQuery({
    query: TagsQuery,
    pause: !showTagModal || !selectedKind,
  });
  const availableKinds = kindsData?.tagKinds || [];
  const availableTags = (kindTagsData?.tags || []).filter((t) => t.kind === selectedKind);

  const tags = post?.tags || [];
  const tagsByKind: Record<string, typeof tags> = {};
  for (const tag of tags) {
    if (!tagsByKind[tag.kind]) tagsByKind[tag.kind] = [];
    tagsByKind[tag.kind].push(tag);
  }

  const mutationContext = { additionalTypenames: ["Post", "PostConnection", "PostStats"] };

  // ---------------------------------------------------------------------------
  // Inline update helper — fires updatePost for a single field change
  // ---------------------------------------------------------------------------
  const inlineUpdate = useCallback(
    async (input: Record<string, unknown>) => {
      await updatePost({ id: postId, input }, mutationContext);
    },
    [postId, updatePost]
  );

  // ---------------------------------------------------------------------------
  // Action handlers (same as before)
  // ---------------------------------------------------------------------------

  const handleAddTag = async () => {
    if (!postId || !selectedKind || !tagValue) return;
    setIsUpdating(true);
    try {
      await addPostTag({ postId, tagKind: selectedKind, tagValue, displayName: tagDisplayName || tagValue }, mutationContext);
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

  const withAction = (name: string, fn: () => Promise<unknown>) => async () => {
    setActionInProgress(name);
    try { await fn(); } catch (err) { console.error(`Failed to ${name}:`, err); } finally { setActionInProgress(null); }
  };

  const handleRegenerate = withAction("regenerate", () => regeneratePost({ id: postId }, mutationContext));
  const handleRegenerateTags = withAction("regenerate_tags", () => regeneratePostTags({ id: postId }, mutationContext));
  const handleArchive = withAction("archive", () => archivePost({ id: postId }, mutationContext));
  const handleReactivate = withAction("reactivate", () => reactivatePost({ id: postId }, mutationContext));
  const handleApprove = withAction("approve", () => approvePost({ id: postId }, mutationContext));
  const handleReject = withAction("reject", () => rejectPost({ id: postId, reason: "Rejected by admin" }, mutationContext));
  const handleDelete = withAction("delete", async () => {
    await deletePost({ id: postId }, mutationContext);
    router.push("/admin/posts");
  });

  // ---------------------------------------------------------------------------
  // Loading / error states
  // ---------------------------------------------------------------------------

  if (isLoading) return <AdminLoader label="Loading post..." />;

  if (error) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-danger-text mb-4">Error Loading Post</h1>
          <p className="text-muted-foreground mb-4">{error.message}</p>
          <Link href="/admin/posts" className="text-link hover:text-link-hover">Back to Posts</Link>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-background p-6">
        <div className="max-w-4xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-foreground mb-4">Post Not Found</h1>
          <Link href="/admin/posts" className="text-link hover:text-link-hover">Back to Posts</Link>
        </div>
      </div>
    );
  }

  // Missing fields
  const missingFields: string[] = [];
  if (!post.sourceUrl) missingFields.push("source URL");
  if (!post.location) missingFields.push("location");
  if (tags.length === 0) missingFields.push("tags");
  if (!post.contacts || post.contacts.length === 0) missingFields.push("contact info");
  if (!post.zipCode) missingFields.push("zip code");

  const isUrgent = post.urgency === "urgent";

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="min-h-screen bg-background px-4 py-4">
      <div className="max-w-7xl mx-auto">

        {/* ── Header bar ─────────────────────────────────────────────── */}
        <div className="flex items-center justify-between mb-4">
          <Link
            href="/admin/posts"
            className="inline-flex items-center text-muted-foreground hover:text-foreground text-sm"
          >
            {"\u2190"} Back to Posts
          </Link>

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

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm" disabled={actionInProgress !== null}>
                  {actionInProgress ? "..." : "\u22EF"}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onSelect={() => setShowTagModal(true)}>Edit Tags</DropdownMenuItem>
                <DropdownMenuItem onSelect={handleRegenerateTags} disabled={actionInProgress !== null}>
                  {actionInProgress === "regenerate_tags" ? "Regenerating..." : "Regenerate Tags"}
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={handleRegenerate} disabled={actionInProgress !== null}>
                  {actionInProgress === "regenerate" ? "Re-running..." : "Re-run Investigation"}
                </DropdownMenuItem>
                {post.status === "active" && (
                  <DropdownMenuItem onSelect={handleArchive} disabled={actionInProgress !== null}>
                    {actionInProgress === "archive" ? "Archiving..." : "Archive (Delist)"}
                  </DropdownMenuItem>
                )}
                <DropdownMenuSeparator />
                <DropdownMenuItem variant="destructive" onSelect={handleDelete} disabled={actionInProgress !== null}>
                  Delete Post
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>

        {/* Missing fields warning */}
        {missingFields.length > 0 && (
          <Alert variant="warning" className="mb-4">
            <AlertDescription>
              <span className="font-medium">Missing fields: </span>
              {missingFields.join(", ")}
            </AlertDescription>
          </Alert>
        )}

        {/* ── Two-column layout ──────────────────────────────────────── */}
        <div className="grid grid-cols-1 lg:grid-cols-[3fr_2fr] gap-6">

          {/* ── LEFT COLUMN (60%) ──────────────────────────────────── */}
          <div className="space-y-6">

            {/* Title */}
            <h1 className="text-2xl font-bold text-foreground">{post.title}</h1>

            {/* Inline-editable metadata */}
            <div className="space-y-3 border-t border-border pt-4">
              <SectionLabel>Post Metadata</SectionLabel>

              <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                {/* Post Type */}
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Type</label>
                  <Select
                    value={post.postType || "notice"}
                    onValueChange={(v) => inlineUpdate({ postType: v })}
                  >
                    <SelectTrigger className="h-8 text-sm w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {POST_TYPES.map((t) => (
                        <SelectItem key={t.value} value={t.value}>{t.label}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {/* Weight */}
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Weight</label>
                  <Select
                    value={post.weight || "medium"}
                    onValueChange={(v) => inlineUpdate({ weight: v })}
                  >
                    <SelectTrigger className="h-8 text-sm w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {WEIGHTS.map((w) => (
                        <SelectItem key={w.value} value={w.value}>{w.label}</SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {/* Priority */}
                <div>
                  <label className="block text-xs text-muted-foreground uppercase mb-1">Priority</label>
                  <Input
                    type="number"
                    defaultValue={post.priority ?? 0}
                    className="h-8 text-sm"
                    onBlur={(e) => {
                      const val = Number(e.target.value);
                      if (val !== (post.priority ?? 0)) {
                        inlineUpdate({ priority: val });
                      }
                    }}
                  />
                </div>
              </div>

              {/* Organization */}
              <div>
                <label className="block text-xs text-muted-foreground uppercase mb-1">Organization</label>
                <Select
                  value={post.organizationId || "__none__"}
                  onValueChange={(v) => {
                    // Organization is not in UpdatePostInput — this is display-only for now
                    // TODO: Add organizationId to UpdatePostInput when needed
                  }}
                  disabled
                >
                  <SelectTrigger className="h-8 text-sm w-full max-w-sm">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="__none__">None</SelectItem>
                    {organizations.map((org) => (
                      <SelectItem key={org.id} value={org.id}>{org.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* Location + Zip Code */}
              <div className="grid grid-cols-2 gap-3">
                <InlineTextField
                  label="Location"
                  value={post.location || ""}
                  placeholder="e.g. Minneapolis, MN"
                  onSave={(v) => inlineUpdate({ location: v || null })}
                />
                <InlineTextField
                  label="Zip Code"
                  value={post.zipCode || ""}
                  placeholder="e.g. 55401"
                  onSave={(v) => inlineUpdate({ zipCode: v || null })}
                />
              </div>
            </div>

            {/* Full text */}
            <div className="border-t border-border pt-4">
              <SectionLabel>Full Text</SectionLabel>
              <div className="prose prose-stone max-w-none text-sm">
                <ReactMarkdown components={markdownComponents}>
                  {post.descriptionMarkdown || post.description || ""}
                </ReactMarkdown>
              </div>
            </div>

            {/* Body previews — heavy / medium / light */}
            <BodyPreview label="Heavy" text={post.bodyHeavy} />
            <BodyPreview label="Medium" text={post.bodyMedium} />
            <BodyPreview label="Light" text={post.bodyLight} />
          </div>

          {/* ── RIGHT COLUMN (40%) ─────────────────────────────────── */}
          <div className="space-y-6">

            {/* Metadata */}
            <div>
              <SectionLabel>Details</SectionLabel>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Submitted by</span>
                  <span className="text-foreground font-medium">
                    {post.submittedBy?.submitterType === "agent" && post.submittedBy.agentId ? (
                      <Link href={`/admin/agents/${post.submittedBy.agentId}`} className="text-link hover:text-link-hover">
                        {post.submittedBy.agentName || "Agent"} (AI)
                      </Link>
                    ) : post.submittedBy?.submitterType === "member" ? (
                      "Member"
                    ) : (
                      <span className="text-text-faint">Unknown</span>
                    )}
                  </span>
                </div>
                {post.sourceUrl && (
                  <div className="flex justify-between gap-4">
                    <span className="text-muted-foreground shrink-0">Source URL</span>
                    <a
                      href={post.sourceUrl.startsWith("http") ? post.sourceUrl : `https://${post.sourceUrl}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-link hover:text-link-hover truncate text-right"
                    >
                      {post.sourceUrl}
                    </a>
                  </div>
                )}
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Ingested</span>
                  <span className="text-foreground">{formatDate(post.createdAt)}</span>
                </div>
                {post.publishedAt && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Published</span>
                    <span className="text-foreground">{formatDate(post.publishedAt)}</span>
                  </div>
                )}
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Last edited</span>
                  <span className="text-foreground">{formatDate(post.updatedAt)}</span>
                </div>
              </div>
            </div>

            {/* Urgent toggle */}
            <div className="border-t border-border pt-4">
              <div className="flex items-center justify-between">
                <SectionLabel>Urgent</SectionLabel>
                <button
                  onClick={() => inlineUpdate({ urgency: isUrgent ? "" : "urgent" })}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 ${
                    isUrgent ? "bg-red-500" : "bg-secondary border border-border"
                  }`}
                  role="switch"
                  aria-checked={isUrgent}
                  aria-label="Toggle urgent"
                >
                  <span
                    className={`inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform ${
                      isUrgent ? "translate-x-6" : "translate-x-1"
                    }`}
                  />
                </button>
              </div>
              {isUrgent && (
                <p className="text-xs text-red-600 mt-1">This post is flagged as urgent and will display an Urgent label in the broadsheet.</p>
              )}
            </div>

            {/* Tags */}
            <div className="border-t border-border pt-4">
              <div className="flex justify-between items-center mb-3">
                <SectionLabel>Tags</SectionLabel>
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

            {/* Contacts */}
            {post.contacts && post.contacts.length > 0 && (
              <div className="border-t border-border pt-4">
                <SectionLabel>Contacts</SectionLabel>
                <div className="space-y-2">
                  {post.contacts.map((c) => (
                    <div key={c.id} className="flex items-start gap-3">
                      <span className="text-xs text-muted-foreground uppercase w-16 flex-shrink-0 pt-0.5">{c.contactType}</span>
                      <span className="text-sm text-text-body break-all">
                        {c.contactType === "email" ? (
                          <a href={`mailto:${c.contactValue}`} className="text-link hover:text-link-hover">{c.contactValue}</a>
                        ) : c.contactType === "phone" ? (
                          <a href={`tel:${c.contactValue}`} className="text-link hover:text-link-hover">{c.contactValue}</a>
                        ) : c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social" || c.contactType === "intake_form_url" ? (
                          <a href={c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`} target="_blank" rel="noopener noreferrer" className="text-link hover:text-link-hover">{c.contactValue}</a>
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

            {/* Schedule */}
            {post.schedules && post.schedules.length > 0 && (() => {
              const oneOffSchedules = post.schedules!.filter((s) => !s.rrule);
              const allOneOffsExpired = oneOffSchedules.length > 0 && oneOffSchedules.every(isScheduleExpired);
              return (
                <div className="border-t border-border pt-4">
                  <SectionLabel>Schedule</SectionLabel>
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

            {/* Notes */}
            {notes.length > 0 && (
              <div className="border-t border-border pt-4">
                <SectionLabel>Notes ({notes.length})</SectionLabel>
                <div className="space-y-2">
                  {notes.map((note) => {
                    const isExpired = !!note.expiredAt;
                    const severityVariant: "danger" | "warning" | "info" =
                      note.severity === "urgent" ? "danger" :
                      note.severity === "notice" ? "warning" : "info";
                    return (
                      <div
                        key={note.id}
                        className={`p-3 rounded-lg border ${isExpired ? "border-border bg-secondary opacity-60" : "border-border"}`}
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
                          <a href={note.sourceUrl} target="_blank" rel="noopener noreferrer" className="text-xs text-link hover:text-link-hover mt-1 inline-block">
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
        </div>
      </div>

      {/* ── Tag Editor Modal ─────────────────────────────────────── */}
      <Dialog open={showTagModal} onOpenChange={setShowTagModal}>
        <DialogContent className="max-w-lg max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>Edit Tags</DialogTitle>
          </DialogHeader>

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
                    <option key={kind.id} value={kind.slug}>{kind.displayName}</option>
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
                          onClick={() => { setIsCreatingNewTag(false); setTagValue(""); setTagDisplayName(""); }}
                        >
                          Back to list
                        </Button>
                      </div>
                    ) : (
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
                          <option key={tag.id} value={tag.value}>{tag.value}</option>
                        ))}
                        <option value="__new__">+ Create new...</option>
                      </select>
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
