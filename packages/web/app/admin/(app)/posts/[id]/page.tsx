"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useRestateObject, useRestate, callObject, callService, invalidateService, invalidateObject } from "@/lib/restate/client";
import { useState, useRef, useEffect } from "react";
import type { PostDetail, TagResult, TagKindListResult, TagListResult, EntityProposalListResult, EntityProposal, PostScheduleResult } from "@/lib/restate/types";

const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

function formatTime12h(time24: string): string {
  const [h, m] = time24.split(":").map(Number);
  const suffix = h >= 12 ? "PM" : "AM";
  const h12 = h % 12 || 12;
  return `${h12}:${m.toString().padStart(2, "0")} ${suffix}`;
}

function formatSchedule(s: PostScheduleResult): string {
  if (s.dtstart && s.day_of_week == null) {
    const date = new Date(s.dtstart);
    const dateStr = date.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
    const timeStr = s.opens_at && s.closes_at
      ? `${formatTime12h(s.opens_at)} – ${formatTime12h(s.closes_at)}`
      : s.opens_at ? formatTime12h(s.opens_at) : "";
    const parts = [dateStr, timeStr].filter(Boolean).join("  ");
    return s.notes ? `${parts} (${s.notes})` : parts;
  }

  const dayName = s.day_of_week != null ? DAY_NAMES[s.day_of_week] : "";
  const timeStr = s.opens_at && s.closes_at
    ? `${formatTime12h(s.opens_at)} – ${formatTime12h(s.closes_at)}`
    : s.opens_at ? formatTime12h(s.opens_at) : "";

  let suffix = "";
  if (s.rrule?.includes("INTERVAL=2")) suffix = " (every other week)";
  if (s.rrule?.includes("FREQ=MONTHLY")) suffix = " (monthly)";
  if (s.notes) suffix = ` (${s.notes})`;

  return [dayName, timeStr, suffix].filter(Boolean).join("  ");
}

export default function PostDetailPage() {
  const params = useParams();
  const router = useRouter();
  const postId = params.id as string;
  const [isUpdating, setIsUpdating] = useState(false);
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());
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

  const { data: post, isLoading, error, mutate: refetch } = useRestateObject<PostDetail>(
    "Post", postId, "get", {},
    { revalidateOnFocus: false }
  );

  const { data: proposalsData, mutate: refetchProposals } = useRestate<EntityProposalListResult>(
    "Sync", "list_entity_proposals",
    { entity_id: postId },
    { revalidateOnFocus: false }
  );

  const proposals = proposalsData?.proposals || [];

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
  const tagsByKind: Record<string, TagResult[]> = {};
  for (const tag of tags) {
    if (!tagsByKind[tag.kind]) tagsByKind[tag.kind] = [];
    tagsByKind[tag.kind].push(tag);
  }

  // Load tag kinds and tags for the selected kind in the modal
  const { data: kindsData } = useRestate<TagKindListResult>(
    showTagModal ? "Tags" : null, "list_kinds", {}
  );
  const { data: kindTagsData } = useRestate<TagListResult>(
    showTagModal && selectedKind ? "Tags" : null,
    "list_tags",
    { kind: selectedKind }
  );

  const availableKinds = kindsData?.kinds || [];
  const availableTags = kindTagsData?.tags || [];

  const handleAddTag = async () => {
    if (!postId || !selectedKind || !tagValue) return;
    setIsUpdating(true);
    try {
      await callObject("Post", postId, "add_tag", {
        tag_kind: selectedKind,
        tag_value: tagValue,
        display_name: tagDisplayName || tagValue,
      });
      setTagValue("");
      setTagDisplayName("");
      invalidateObject("Post", postId);
      refetch();
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
      await callObject("Post", postId, "remove_tag", { tag_id: tagId });
      invalidateObject("Post", postId);
      refetch();
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
      await callObject("Post", postId, "regenerate", {});
      invalidateObject("Post", postId);
      refetch();
    } catch (err) {
      console.error("Failed to regenerate post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleDelete = async () => {
    setActionInProgress("delete");
    setMenuOpen(false);
    try {
      await callObject("Post", postId, "delete", {});
      router.push("/admin/posts");
    } catch (err) {
      console.error("Failed to delete post:", err);
      setActionInProgress(null);
    }
  };

  const handleApprove = async () => {
    setActionInProgress("approve");
    try {
      await callObject("Post", postId, "approve", {});
      invalidateService("Posts");
      invalidateObject("Post", postId);
      refetch();
    } catch (err) {
      console.error("Failed to approve post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    setActionInProgress("reject");
    try {
      await callObject("Post", postId, "reject", { reason: "Rejected by admin" });
      invalidateService("Posts");
      invalidateObject("Post", postId);
      refetch();
    } catch (err) {
      console.error("Failed to reject post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleApproveProposal = async (proposalId: string) => {
    try {
      await callService("Sync", "approve_proposal", { proposal_id: proposalId });
      invalidateService("Sync");
      invalidateObject("Post", postId);
      refetchProposals();
      refetch();
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
  if (!post.source_url && post.website_id) missingFields.push("source URL");
  if (!post.tldr) missingFields.push("TLDR");
  if (!post.location) missingFields.push("location");
  if (tags.length === 0) missingFields.push("tags");

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
              {post.status === "pending_approval" && (
                <>
                  <button
                    onClick={handleApprove}
                    disabled={actionInProgress !== null}
                    className="px-4 py-1.5 bg-emerald-400 text-white text-sm rounded-full font-medium hover:bg-emerald-500 transition-colors disabled:opacity-50"
                  >
                    {actionInProgress === "approve" ? "..." : "Approve"}
                  </button>
                  <button
                    onClick={handleReject}
                    disabled={actionInProgress !== null}
                    className="px-4 py-1.5 bg-rose-400 text-white text-sm rounded-full font-medium hover:bg-rose-500 transition-colors disabled:opacity-50"
                  >
                    {actionInProgress === "reject" ? "..." : "Reject"}
                  </button>
                </>
              )}
              <span
                className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusBadgeClass(post.status)}`}
              >
                {post.status.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase())}
              </span>

              {post.source_url && (
                <a
                  href={post.source_url.startsWith("http") ? post.source_url : `https://${post.source_url}`}
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
                      onClick={handleRegenerate}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      {actionInProgress === "regenerate" ? "Re-running..." : "Re-run Investigation"}
                    </button>
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

          {post.tldr && (
            <p className="text-stone-700 bg-amber-50 p-3 rounded-lg mb-4">{post.tldr}</p>
          )}

          {/* Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Type</span>
              <p className="text-sm font-medium text-stone-900">{post.post_type}</p>
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
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(post.created_at)}</p>
            </div>
            {post.source_url && (
              <div className="col-span-2">
                <span className="text-xs text-stone-500 uppercase">Source URL</span>
                <p className="text-sm font-medium truncate">
                  <a
                    href={post.source_url.startsWith("http") ? post.source_url : `https://${post.source_url}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:text-blue-800"
                  >
                    {post.source_url}
                  </a>
                </p>
              </div>
            )}
            {post.website_id && (
              <div>
                <span className="text-xs text-stone-500 uppercase">Website</span>
                <p className="text-sm font-medium">
                  <Link href={`/admin/websites/${post.website_id}`} className="text-blue-600 hover:text-blue-800">
                    View Website {"\u2192"}
                  </Link>
                </p>
              </div>
            )}
            <div>
              <span className="text-xs text-stone-500 uppercase">Submitted By</span>
              <p className="text-sm font-medium text-stone-900">
                {post.submitted_by?.submitter_type === "agent" && post.submitted_by.agent_id ? (
                  <Link href={`/admin/agents/${post.submitted_by.agent_id}`} className="text-purple-600 hover:text-purple-800">
                    {post.submitted_by.agent_name || "Agent"} (AI)
                  </Link>
                ) : post.submitted_by?.submitter_type === "member" ? (
                  <span>Member</span>
                ) : (
                  <span className="text-stone-400">Unknown</span>
                )}
              </p>
            </div>
          </div>
        </div>

        {/* Pending Changes (Proposals) */}
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
                    {kindTags.map((tag: TagResult) => (
                      <span
                        key={tag.id}
                        className={`px-3 py-1 text-sm rounded-full font-medium ${!tag.color ? "bg-stone-100 text-stone-800" : ""}`}
                        style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
                      >
                        {tag.display_name || tag.value}
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
        {post.schedules && post.schedules.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">Schedule</h2>
            <div className="space-y-2">
              {post.schedules.map((s: PostScheduleResult) => (
                <div key={s.id} className="flex items-start gap-2 text-stone-700">
                  <svg className="w-4 h-4 mt-0.5 flex-shrink-0 text-stone-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span className="text-sm">{formatSchedule(s)}</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Source Pages */}
        {post.source_pages && post.source_pages.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">
              Source Pages ({post.source_pages.length})
            </h2>
            <p className="text-sm text-stone-500 mb-4">
              Pages from which this post was extracted.
            </p>
            <div className="space-y-3">
              {post.source_pages.map((page) => {
                const isExpanded = expandedPages.has(page.url);
                return (
                  <div key={page.url} className="border border-stone-200 rounded-lg">
                    <div className="flex items-center justify-between p-4">
                      <div className="flex-1 min-w-0">
                        <a
                          href={page.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-sm font-medium text-blue-600 hover:text-blue-800 truncate block"
                        >
                          {page.title || page.url}
                        </a>
                        <p className="text-xs text-stone-400 truncate mt-1">{page.url}</p>
                        <p className="text-xs text-stone-400 mt-1">
                          Fetched {formatDate(page.fetched_at)}
                        </p>
                      </div>
                      <button
                        onClick={() => {
                          const next = new Set(expandedPages);
                          if (isExpanded) {
                            next.delete(page.url);
                          } else {
                            next.add(page.url);
                          }
                          setExpandedPages(next);
                        }}
                        className="ml-4 px-3 py-1 text-xs text-stone-500 hover:text-stone-700 border border-stone-200 rounded hover:bg-stone-50"
                      >
                        {isExpanded ? "Hide content" : "Show content"}
                      </button>
                    </div>
                    {isExpanded && (
                      <div className="border-t border-stone-200 p-4 bg-stone-50 max-h-96 overflow-y-auto">
                        <div className="prose prose-sm prose-stone max-w-none">
                          <ReactMarkdown
                            components={{
                              p: ({ children }) => <p className="mb-2 text-stone-600 text-sm">{children}</p>,
                              ul: ({ children }) => <ul className="list-disc pl-5 mb-2 space-y-0.5">{children}</ul>,
                              ol: ({ children }) => <ol className="list-decimal pl-5 mb-2 space-y-0.5">{children}</ol>,
                              li: ({ children }) => <li className="text-stone-600 text-sm">{children}</li>,
                              strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
                              a: ({ href, children }) => (
                                <a href={href} className="text-blue-600 hover:text-blue-800 underline" target="_blank" rel="noopener noreferrer">
                                  {children}
                                </a>
                              ),
                              h1: ({ children }) => <h1 className="text-base font-bold text-stone-800 mt-3 mb-1">{children}</h1>,
                              h2: ({ children }) => <h2 className="text-sm font-bold text-stone-800 mt-3 mb-1">{children}</h2>,
                              h3: ({ children }) => <h3 className="text-sm font-semibold text-stone-700 mt-2 mb-1">{children}</h3>,
                            }}
                          >
                            {page.content}
                          </ReactMarkdown>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Description */}
        <div className="bg-white rounded-lg shadow-md p-6">
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
              {post.description_markdown || post.description || ""}
            </ReactMarkdown>
          </div>
        </div>
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
                      {kindTags.map((tag: TagResult) => (
                        <span
                          key={tag.id}
                          className={`inline-flex items-center gap-1 px-3 py-1 text-sm rounded-full font-medium ${!tag.color ? "bg-stone-100 text-stone-800" : ""}`}
                          style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
                        >
                          {tag.display_name || tag.value}
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
                        {kind.display_name}
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
                              setTagDisplayName(match?.display_name || val);
                            }}
                            className="w-full px-3 py-2 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                          >
                            <option value="">Select a value...</option>
                            {availableTags.map((tag) => (
                              <option key={tag.id} value={tag.value}>
                                {tag.display_name || tag.value}
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
