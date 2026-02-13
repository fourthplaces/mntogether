"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useQuery, useMutation } from "urql";
import { useRestate, callService, invalidateService, invalidateObject } from "@/lib/restate/client";
import { useState, useRef, useEffect } from "react";
import type { EntityProposalListResult, EntityProposal, NoteListResult, NoteResult } from "@/lib/restate/types";
import {
  PostDetailQuery,
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
  const [menuOpen, setMenuOpen] = useState(false);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [showTagModal, setShowTagModal] = useState(false);
  const [selectedKind, setSelectedKind] = useState("");
  const [tagValue, setTagValue] = useState("");
  const [tagDisplayName, setTagDisplayName] = useState("");
  const [isCreatingNewTag, setIsCreatingNewTag] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  // GraphQL: fetch post detail
  const [{ data: postData, fetching: isLoading, error }] = useQuery({
    query: PostDetailQuery,
    variables: { id: postId },
  });
  const post = postData?.post;

  // Restate: proposals (Sync domain — will migrate in Phase 7)
  const { data: proposalsData, mutate: refetchProposals } = useRestate<EntityProposalListResult>(
    "Sync", "list_entity_proposals",
    { entity_id: postId },
    { revalidateOnFocus: false }
  );

  // Restate: notes (Notes domain — will migrate in a later phase)
  const { data: notesData } = useRestate<NoteListResult>(
    "Notes", "list_for_entity",
    { noteable_type: "post", noteable_id: postId },
    { revalidateOnFocus: false }
  );

  const proposals = proposalsData?.proposals || [];
  const notes = notesData?.notes || [];

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

  const getStatusBadgeClass = (status: string) => {
    switch (status) {
      case "active":
        return "bg-green-100 text-green-800";
      case "pending_approval":
        return "bg-amber-100 text-amber-800";
      case "rejected":
        return "bg-red-100 text-red-800";
      default:
        return "bg-stone-100 text-stone-800";
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
    setMenuOpen(false);
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
    setMenuOpen(false);
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
    setMenuOpen(false);
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
    setMenuOpen(false);
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

  // Proposals still use Restate (Phase 7)
  const handleApproveProposal = async (proposalId: string) => {
    try {
      await callService("Sync", "approve_proposal", { proposal_id: proposalId });
      invalidateService("Sync");
      invalidateObject("Post", postId);
      refetchProposals();
    } catch (err) {
      console.error("Failed to approve proposal:", err);
    }
  };

  const handleRejectProposal = async (proposalId: string) => {
    try {
      await callService("Sync", "reject_proposal", { proposal_id: proposalId });
      invalidateService("Sync");
      refetchProposals();
    } catch (err) {
      console.error("Failed to reject proposal:", err);
    }
  };

  if (isLoading) {
    return <AdminLoader label="Loading post..." />;
  }

  if (error) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Post</h1>
            <p className="text-stone-600 mb-4">{error.message}</p>
            <Link href="/admin/posts" className="text-blue-600 hover:text-blue-800">
              Back to Posts
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Post Not Found</h1>
            <Link href="/admin/posts" className="text-blue-600 hover:text-blue-800">
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
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-4xl mx-auto">
        {/* Back Button */}
        <Link
          href="/admin/posts"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Posts
        </Link>

        {/* Post Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start mb-4">
            <div className="flex-1">
              <h1 className="text-2xl font-bold text-stone-900 mb-2">{post.title}</h1>
            </div>
            <div className="flex items-center gap-2">
              <select
                value={post.status}
                disabled={actionInProgress !== null}
                onChange={(e) => {
                  const newStatus = e.target.value;
                  if (newStatus === post.status) return;
                  if (newStatus === "active") handleApprove();
                  else if (newStatus === "rejected") handleReject();
                  else if (newStatus === "archived") handleArchive();
                  else if (newStatus === "pending_approval") handleReactivate();
                }}
                className={`pl-2.5 py-1 text-xs rounded-full font-medium appearance-none cursor-pointer pr-5 border-0 ${getStatusBadgeClass(post.status)} disabled:opacity-50`}
                style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10' viewBox='0 0 12 12'%3E%3Cpath fill='%23666' d='M3 5l3 3 3-3'/%3E%3C/svg%3E")`, backgroundRepeat: "no-repeat", backgroundPosition: "right 6px center" }}
              >
                <option value="pending_approval">Pending</option>
                <option value="active">Active</option>
                <option value="rejected">Rejected</option>
                <option value="archived">Archived</option>
              </select>

              {post.status === "active" && (
                <Link
                  href={`/posts/${postId}`}
                  className="p-2 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded-lg"
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
                  className="p-2 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded-lg"
                  title="View source page"
                >
                  {"\u{1F517}"}
                </a>
              )}

              {/* More Actions Dropdown */}
              <div className="relative" ref={menuRef}>
                <button
                  onClick={() => setMenuOpen(!menuOpen)}
                  disabled={actionInProgress !== null}
                  className="px-3 py-2 bg-stone-100 text-stone-700 rounded hover:bg-stone-200 disabled:opacity-50"
                >
                  {actionInProgress ? "..." : "\u22EF"}
                </button>
                {menuOpen && (
                  <div className="absolute right-0 mt-2 w-48 bg-white rounded-lg shadow-lg border border-stone-200 py-1 z-10">
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        setShowTagModal(true);
                      }}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50"
                    >
                      Edit Tags
                    </button>
                    <button
                      onClick={handleRegenerateTags}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      {actionInProgress === "regenerate_tags" ? "Regenerating..." : "Regenerate Tags"}
                    </button>
                    <button
                      onClick={handleRegenerate}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      {actionInProgress === "regenerate" ? "Re-running..." : "Re-run Investigation"}
                    </button>
                    {post.status === "active" && (
                      <button
                        onClick={handleArchive}
                        disabled={actionInProgress !== null}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {actionInProgress === "archive" ? "Archiving..." : "Archive (Delist)"}
                      </button>
                    )}
                    <div className="border-t border-stone-100 my-1" />
                    <button
                      onClick={handleDelete}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
                    >
                      Delete Post
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Missing Fields Warning */}
          {missingFields.length > 0 && (
            <div className="mb-4 p-3 bg-amber-50 border border-amber-200 rounded-lg">
              <span className="text-sm font-medium text-amber-800">Missing fields: </span>
              <span className="text-sm text-amber-700">{missingFields.join(", ")}</span>
            </div>
          )}

          {/* Relevance Score */}
          {post.relevanceScore != null && (
            <div className="mb-4 p-3 rounded-lg border border-stone-200 bg-stone-50">
              <div className="flex items-center gap-2 mb-1">
                <span className="text-xs text-stone-500 uppercase font-medium">Relevance Score</span>
                <span
                  className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-bold ${
                    post.relevanceScore >= 8
                      ? "bg-green-100 text-green-800"
                      : post.relevanceScore >= 5
                        ? "bg-amber-100 text-amber-800"
                        : "bg-red-100 text-red-800"
                  }`}
                >
                  {post.relevanceScore}/10
                </span>
                <span className="text-xs text-stone-400">
                  {post.relevanceScore >= 8 ? "High confidence" : post.relevanceScore >= 5 ? "Review needed" : "Likely noise"}
                </span>
              </div>
              {post.relevanceBreakdown && (
                <p className="text-xs text-stone-600 leading-relaxed whitespace-pre-line mt-1">{post.relevanceBreakdown}</p>
              )}
            </div>
          )}

          {/* Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Type</span>
              <p className="text-sm font-medium text-stone-900">{post.postType}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Category</span>
              <p className="text-sm font-medium text-stone-900">{post.category}</p>
            </div>
            {post.urgency && (
              <div>
                <span className="text-xs text-stone-500 uppercase">Urgency</span>
                <p className="text-sm font-medium text-stone-900">{post.urgency}</p>
              </div>
            )}
            {post.location && (
              <div>
                <span className="text-xs text-stone-500 uppercase">Location</span>
                <p className="text-sm font-medium text-stone-900">{post.location}</p>
              </div>
            )}
            <div>
              <span className="text-xs text-stone-500 uppercase">{post.publishedAt ? "Published" : "Created"}</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(post.publishedAt || post.createdAt)}</p>
            </div>
            {post.sourceUrl && (
              <div className="col-span-2">
                <span className="text-xs text-stone-500 uppercase">Source URL</span>
                <p className="text-sm font-medium truncate">
                  <a
                    href={post.sourceUrl.startsWith("http") ? post.sourceUrl : `https://${post.sourceUrl}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:text-blue-800"
                  >
                    {post.sourceUrl}
                  </a>
                </p>
              </div>
            )}
            <div>
              <span className="text-xs text-stone-500 uppercase">Organization</span>
              <p className="text-sm font-medium text-stone-900">
                {post.organizationId ? (
                  <Link href={`/admin/organizations/${post.organizationId}`} className="text-amber-700 hover:text-amber-900">
                    {post.organizationName}
                  </Link>
                ) : (
                  <span className="text-stone-400">None</span>
                )}
              </p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Submitted By</span>
              <p className="text-sm font-medium text-stone-900">
                {post.submittedBy?.submitterType === "agent" && post.submittedBy.agentId ? (
                  <Link href={`/admin/agents/${post.submittedBy.agentId}`} className="text-purple-600 hover:text-purple-800">
                    {post.submittedBy.agentName || "Agent"} (AI)
                  </Link>
                ) : post.submittedBy?.submitterType === "member" ? (
                  <span>Member</span>
                ) : (
                  <span className="text-stone-400">Unknown</span>
                )}
              </p>
            </div>
          </div>
        </div>

        {/* Contact Info */}
        {post.contacts && post.contacts.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">Contact Info</h2>
            <div className="space-y-2">
              {post.contacts.map((c) => (
                <div key={c.id} className="flex items-start gap-3">
                  <span className="text-xs text-stone-500 uppercase w-20 flex-shrink-0 pt-0.5">{c.contactType}</span>
                  <span className="text-sm text-stone-700">
                    {c.contactType === "email" ? (
                      <a href={`mailto:${c.contactValue}`} className="text-blue-600 hover:text-blue-800">{c.contactValue}</a>
                    ) : c.contactType === "phone" ? (
                      <a href={`tel:${c.contactValue}`} className="text-blue-600 hover:text-blue-800">{c.contactValue}</a>
                    ) : c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social" || c.contactType === "intake_form_url" ? (
                      <a href={c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`} target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:text-blue-800 break-all">{c.contactValue}</a>
                    ) : (
                      <span>{c.contactValue}</span>
                    )}
                    {c.contactLabel && <span className="text-stone-400 ml-2">({c.contactLabel})</span>}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Review: pending proposals — still uses Restate (Phase 7) */}
        {proposals.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6 border-l-4 border-amber-400">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">
              Pending Changes ({proposals.length})
            </h2>
            <div className="space-y-3">
                {proposals.map((proposal: EntityProposal) => (
                  <div
                    key={proposal.id}
                    className="flex items-center justify-between border border-stone-200 rounded-lg p-4"
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span
                          className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                            proposal.operation === "create"
                              ? "bg-green-100 text-green-800"
                              : proposal.operation === "update"
                                ? "bg-blue-100 text-blue-800"
                                : proposal.operation === "delete"
                                  ? "bg-red-100 text-red-800"
                                  : proposal.operation === "merge"
                                    ? "bg-purple-100 text-purple-800"
                                    : "bg-stone-100 text-stone-800"
                          }`}
                        >
                          {proposal.operation}
                        </span>
                        <span className="text-xs text-stone-400">
                          {new Date(proposal.created_at).toLocaleDateString()}
                        </span>
                      </div>
                      {proposal.reason && (
                        <p className="text-sm text-stone-600">{proposal.reason}</p>
                      )}
                    </div>
                    <div className="flex gap-2 ml-4">
                      <button
                        onClick={() => handleApproveProposal(proposal.id)}
                        className="px-3 py-1 text-sm bg-emerald-400 text-white rounded hover:bg-emerald-500"
                      >
                        Approve
                      </button>
                      <button
                        onClick={() => handleRejectProposal(proposal.id)}
                        className="px-3 py-1 text-sm bg-rose-400 text-white rounded hover:bg-rose-500"
                      >
                        Reject
                      </button>
                    </div>
                  </div>
                ))}
            </div>
          </div>
        )}

        {/* Tags */}
        <div id="tags-section" className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-lg font-semibold text-stone-900">Tags</h2>
            <button
              onClick={() => setShowTagModal(true)}
              className="text-sm text-blue-600 hover:text-blue-800"
            >
              Edit
            </button>
          </div>

          {tags.length > 0 ? (
            <div className="space-y-3">
              {Object.entries(tagsByKind).map(([kind, kindTags]) => (
                <div key={kind}>
                  <span className="text-xs text-stone-500 uppercase">{kind.replace(/_/g, " ")}</span>
                  <div className="flex flex-wrap gap-2 mt-1">
                    {kindTags.map((tag) => (
                      <span
                        key={tag.id}
                        className={`px-3 py-1 text-sm rounded-full font-medium ${!tag.color ? "bg-stone-100 text-stone-800" : ""}`}
                        style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
                      >
                        {tag.value}
                      </span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <span className="text-stone-400 text-sm">No tags</span>
          )}
        </div>

        {/* Schedule */}
        {post.schedules && post.schedules.length > 0 && (() => {
          const oneOffSchedules = post.schedules!.filter((s) => !s.rrule);
          const allOneOffsExpired = oneOffSchedules.length > 0 && oneOffSchedules.every(isScheduleExpired);
          return (
            <div className="bg-white rounded-lg shadow-md p-6 mb-6">
              <h2 className="text-lg font-semibold text-stone-900 mb-4">Schedule</h2>
              {allOneOffsExpired && (
                <div className="mb-3 px-3 py-2 bg-amber-50 border border-amber-200 rounded-lg text-xs font-medium text-amber-800">
                  This event has passed
                </div>
              )}
              <div className="space-y-2">
                {post.schedules!.map((s) => (
                  <div key={s.id} className={`flex items-start gap-2 text-stone-700 ${isScheduleExpired(s) ? "opacity-60" : ""}`}>
                    <svg className="w-4 h-4 mt-0.5 flex-shrink-0 text-stone-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">Description</h2>
          <div className="prose prose-stone max-w-none">
            <ReactMarkdown
              components={{
                p: ({ children }) => <p className="mb-4 text-stone-700">{children}</p>,
                ul: ({ children }) => <ul className="list-disc pl-6 mb-4 space-y-1">{children}</ul>,
                ol: ({ children }) => <ol className="list-decimal pl-6 mb-4 space-y-1">{children}</ol>,
                li: ({ children }) => <li className="text-stone-700">{children}</li>,
                strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
                a: ({ href, children }) => (
                  <a href={href} className="text-blue-600 hover:text-blue-800 underline" target="_blank" rel="noopener noreferrer">
                    {children}
                  </a>
                ),
              }}
            >
              {post.descriptionMarkdown || post.description || ""}
            </ReactMarkdown>
          </div>
        </div>

        {/* Notes — still uses Restate (later phase) */}
        {notes.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">
              Notes ({notes.length})
            </h2>
            <div className="space-y-2">
              {notes.map((note: NoteResult) => {
                const isExpired = !!note.expired_at;
                const severityStyle =
                  note.severity === "urgent" ? "bg-red-100 text-red-800" :
                  note.severity === "notice" ? "bg-yellow-100 text-yellow-800" :
                  "bg-blue-100 text-blue-800";

                return (
                  <div
                    key={note.id}
                    className={`p-3 rounded-lg border ${
                      isExpired ? "border-stone-200 bg-stone-50 opacity-60" : "border-stone-200"
                    }`}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${severityStyle}`}>
                        {note.severity}
                      </span>
                      {note.is_public && (
                        <span className="px-2 py-0.5 text-xs rounded-full font-medium bg-green-100 text-green-800">
                          public
                        </span>
                      )}
                      {isExpired && (
                        <span className="px-2 py-0.5 text-xs rounded-full font-medium bg-stone-200 text-stone-600">
                          expired
                        </span>
                      )}
                      {note.source_type && (
                        <span className="text-xs text-stone-400">via {note.source_type}</span>
                      )}
                      <span className="text-xs text-stone-400">
                        {note.created_by} &middot; {new Date(note.created_at).toLocaleDateString()}
                      </span>
                    </div>
                    <p className="text-sm text-stone-700">{note.content}</p>
                    {note.source_url && (
                      <a
                        href={note.source_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-xs text-blue-600 hover:text-blue-800 mt-1 inline-block"
                      >
                        Source {"\u2197"}
                      </a>
                    )}
                    {note.linked_posts && note.linked_posts.filter(p => p.id !== postId).length > 0 && (
                      <div className="flex flex-wrap items-center gap-1 mt-1.5">
                        <span className="text-xs text-stone-400">Also on:</span>
                        {note.linked_posts.filter(p => p.id !== postId).map((p) => (
                          <Link
                            key={p.id}
                            href={`/admin/posts/${p.id}`}
                            className="text-xs px-1.5 py-0.5 bg-stone-100 text-stone-600 rounded hover:bg-stone-200 hover:text-stone-800 transition-colors truncate max-w-[200px]"
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

      {/* Tag Editor Modal */}
      {showTagModal && (
        <div className="fixed inset-0 bg-black/30 backdrop-blur-sm flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg p-6 max-w-lg w-full max-h-[80vh] overflow-y-auto shadow-xl">
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-lg font-semibold text-stone-900">Edit Tags</h3>
              <button
                onClick={() => setShowTagModal(false)}
                className="text-stone-400 hover:text-stone-600 text-xl leading-none"
              >
                &times;
              </button>
            </div>

            {/* Current tags grouped by kind */}
            {tags.length > 0 ? (
              <div className="space-y-3 mb-6">
                {Object.entries(tagsByKind).map(([kind, kindTags]) => (
                  <div key={kind}>
                    <span className="text-xs text-stone-500 uppercase font-medium">{kind.replace(/_/g, " ")}</span>
                    <div className="flex flex-wrap gap-2 mt-1">
                      {kindTags.map((tag) => (
                        <span
                          key={tag.id}
                          className={`inline-flex items-center gap-1 px-3 py-1 text-sm rounded-full font-medium ${!tag.color ? "bg-stone-100 text-stone-800" : ""}`}
                          style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
                        >
                          {tag.value}
                          <button
                            onClick={() => handleRemoveTag(tag.id)}
                            disabled={isUpdating}
                            className="hover:text-red-600 ml-1 disabled:opacity-50"
                            style={tag.color ? { color: tag.color } : { color: "#a8a29e" }}
                          >
                            &times;
                          </button>
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-stone-400 text-sm mb-6">No tags yet.</p>
            )}

            {/* Add tag form */}
            <div className="border-t border-stone-200 pt-4">
              <h4 className="text-sm font-medium text-stone-700 mb-3">Add a tag</h4>
              <div className="space-y-3">
                <div>
                  <label className="block text-xs text-stone-500 mb-1">Kind</label>
                  <select
                    value={selectedKind}
                    onChange={(e) => {
                      setSelectedKind(e.target.value);
                      setTagValue("");
                      setTagDisplayName("");
                      setIsCreatingNewTag(false);
                    }}
                    className="w-full px-3 py-2 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
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
                      <label className="block text-xs text-stone-500 mb-1">Value</label>
                      {isCreatingNewTag ? (
                        <div className="space-y-2">
                          <input
                            value={tagValue}
                            onChange={(e) => setTagValue(e.target.value)}
                            placeholder="New tag value..."
                            className="w-full px-3 py-2 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                            autoFocus
                          />
                          <div>
                            <label className="block text-xs text-stone-500 mb-1">Display Name</label>
                            <input
                              value={tagDisplayName}
                              onChange={(e) => setTagDisplayName(e.target.value)}
                              placeholder="Human-readable name..."
                              className="w-full px-3 py-2 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                            />
                          </div>
                          <button
                            onClick={() => {
                              setIsCreatingNewTag(false);
                              setTagValue("");
                              setTagDisplayName("");
                            }}
                            className="text-xs text-stone-500 hover:text-stone-700"
                          >
                            Back to list
                          </button>
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
                            className="w-full px-3 py-2 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
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

                    <button
                      onClick={handleAddTag}
                      disabled={isUpdating || !tagValue}
                      className="w-full px-4 py-2 bg-blue-600 text-white text-sm rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      {isUpdating ? "Adding..." : "Add Tag"}
                    </button>
                  </>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

    </div>
  );
}
