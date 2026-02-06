"use client";

import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useGraphQL, graphqlMutateClient, invalidateAllMatchingQuery } from "@/lib/graphql/client";
import { GET_POST } from "@/lib/graphql/queries";
import { ADD_POST_TAG, REMOVE_POST_TAG, REGENERATE_POST, DELETE_POST, APPROVE_POST, REJECT_POST } from "@/lib/graphql/mutations";
import { useState, useRef, useEffect } from "react";

interface Tag {
  id: string;
  kind: string;
  value: string;
  displayName: string | null;
}

interface SourcePage {
  url: string;
  title: string | null;
  fetchedAt: string;
  content: string;
}

interface PostDetail {
  id: string;
  organizationName: string;
  title: string;
  tldr: string | null;
  description: string;
  descriptionMarkdown: string | null;
  postType: string;
  category: string;
  urgency: string | null;
  location: string | null;
  status: string;
  sourceUrl: string | null;
  websiteId: string | null;
  createdAt: string;
  tags: Tag[];
  sourcePages: SourcePage[];
}

interface GetPostResult {
  listing: PostDetail | null;
}

const AUDIENCE_ROLES = [
  { value: "recipient", label: "Recipient", description: "People receiving services/benefits" },
  { value: "donor", label: "Donor", description: "People giving money/goods" },
  { value: "volunteer", label: "Volunteer", description: "People giving their time" },
  { value: "participant", label: "Participant", description: "People attending events/groups" },
  { value: "customer", label: "Customer", description: "People buying from immigrant-owned businesses" },
];

export default function PostDetailPage() {
  const params = useParams();
  const router = useRouter();
  const postId = params.id as string;
  const [isEditingTags, setIsEditingTags] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [expandedPages, setExpandedPages] = useState<Set<string>>(new Set());
  const [menuOpen, setMenuOpen] = useState(false);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [showRejectModal, setShowRejectModal] = useState(false);
  const [rejectReason, setRejectReason] = useState("");
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

  const { data, isLoading, error, mutate: refetch } = useGraphQL<GetPostResult>(
    GET_POST,
    { id: postId },
    { revalidateOnFocus: false }
  );

  const post = data?.listing;

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

  const getAudienceRoleBadgeClass = (role: string) => {
    switch (role) {
      case "recipient":
        return "bg-blue-100 text-blue-800";
      case "donor":
        return "bg-green-100 text-green-800";
      case "volunteer":
        return "bg-purple-100 text-purple-800";
      case "participant":
        return "bg-amber-100 text-amber-800";
      case "customer":
        return "bg-teal-100 text-teal-800";
      default:
        return "bg-stone-100 text-stone-800";
    }
  };

  const audienceRoleTags = post?.tags.filter((t) => t.kind === "audience_role") || [];
  const otherTags = post?.tags.filter((t) => t.kind !== "audience_role") || [];

  const handleToggleAudienceRole = async (role: string) => {
    if (!postId) return;

    setIsUpdating(true);
    try {
      const existingTag = audienceRoleTags.find((t) => t.value === role);
      if (existingTag) {
        await graphqlMutateClient(REMOVE_POST_TAG, { listingId: postId, tagId: existingTag.id });
      } else {
        const roleInfo = AUDIENCE_ROLES.find((r) => r.value === role);
        await graphqlMutateClient(ADD_POST_TAG, {
          listingId: postId,
          tagKind: "audience_role",
          tagValue: role,
          displayName: roleInfo?.label || role,
        });
      }
      invalidateAllMatchingQuery(GET_POST);
      refetch();
    } catch (err) {
      console.error("Failed to update tag:", err);

    } finally {
      setIsUpdating(false);
    }
  };

  const handleRegenerate = async () => {
    setActionInProgress("regenerate");
    setMenuOpen(false);
    try {
      await graphqlMutateClient(REGENERATE_POST, { postId });
      refetch();
    } catch (err) {
      console.error("Failed to regenerate post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleDelete = async () => {
    if (!confirm("Are you sure you want to delete this post? This cannot be undone.")) return;

    setActionInProgress("delete");
    setMenuOpen(false);
    try {
      await graphqlMutateClient(DELETE_POST, { listingId: postId });
      router.push("/admin/posts");
    } catch (err) {
      console.error("Failed to delete post:", err);
      setActionInProgress(null);
    }
  };

  const handleApprove = async () => {
    setActionInProgress("approve");
    try {
      await graphqlMutateClient(APPROVE_POST, { listingId: postId });
      refetch();
    } catch (err) {
      console.error("Failed to approve post:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    setActionInProgress("reject");
    setShowRejectModal(false);
    try {
      await graphqlMutateClient(REJECT_POST, { listingId: postId, reason: rejectReason || "Rejected by admin" });
      setRejectReason("");
      refetch();
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
  if (!post.sourceUrl && post.websiteId) missingFields.push("source URL");
  if (!post.tldr) missingFields.push("TLDR");
  if (!post.location) missingFields.push("location");
  if (audienceRoleTags.length === 0) missingFields.push("audience role");

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
              <p className="text-lg text-stone-600">{post.organizationName}</p>
            </div>
            <div className="flex items-center gap-2">
              {post.status === "pending_approval" && (
                <>
                  <button
                    onClick={handleApprove}
                    disabled={actionInProgress !== null}
                    className="px-4 py-1.5 bg-green-600 text-white text-sm rounded-full font-medium hover:bg-green-700 transition-colors disabled:opacity-50"
                  >
                    {actionInProgress === "approve" ? "..." : "Approve"}
                  </button>
                  <button
                    onClick={() => setShowRejectModal(true)}
                    disabled={actionInProgress !== null}
                    className="px-4 py-1.5 bg-red-600 text-white text-sm rounded-full font-medium hover:bg-red-700 transition-colors disabled:opacity-50"
                  >
                    {actionInProgress === "reject" ? "..." : "Reject"}
                  </button>
                </>
              )}
              <span
                className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusBadgeClass(post.status)}`}
              >
                {post.status.replace("_", " ")}
              </span>

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
                    {post.websiteId && post.sourceUrl && (
                      <button
                        onClick={handleRegenerate}
                        disabled={actionInProgress !== null}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        Regenerate
                      </button>
                    )}
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
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(post.createdAt)}</p>
            </div>
            {post.websiteId && (
              <div>
                <span className="text-xs text-stone-500 uppercase">Website</span>
                <p className="text-sm font-medium">
                  <Link href={`/admin/websites/${post.websiteId}`} className="text-blue-600 hover:text-blue-800">
                    View Website {"\u2192"}
                  </Link>
                </p>
              </div>
            )}
          </div>
        </div>

        {/* Audience Roles */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-lg font-semibold text-stone-900">Audience Roles</h2>
            <button
              onClick={() => setIsEditingTags(!isEditingTags)}
              className="text-sm text-blue-600 hover:text-blue-800"
            >
              {isEditingTags ? "Done" : "Edit"}
            </button>
          </div>

          <p className="text-sm text-stone-500 mb-4">Who is this post for? Select all that apply.</p>

          {isEditingTags ? (
            <div className="grid grid-cols-2 gap-3">
              {AUDIENCE_ROLES.map((role) => {
                const isSelected = audienceRoleTags.some((t) => t.value === role.value);
                return (
                  <button
                    key={role.value}
                    onClick={() => handleToggleAudienceRole(role.value)}
                    disabled={isUpdating}
                    className={`p-3 rounded-lg border-2 text-left transition-colors ${
                      isSelected
                        ? "border-blue-500 bg-blue-50"
                        : "border-stone-200 hover:border-stone-300"
                    } ${isUpdating ? "opacity-50 cursor-wait" : ""}`}
                  >
                    <div className="font-medium text-stone-900">{role.label}</div>
                    <div className="text-xs text-stone-500">{role.description}</div>
                  </button>
                );
              })}
            </div>
          ) : (
            <div className="flex flex-wrap gap-2">
              {audienceRoleTags.length > 0 ? (
                audienceRoleTags.map((tag) => (
                  <span
                    key={tag.id}
                    className={`px-3 py-1 text-sm rounded-full font-medium ${getAudienceRoleBadgeClass(tag.value)}`}
                  >
                    {tag.displayName || tag.value}
                  </span>
                ))
              ) : (
                <span className="text-stone-400 text-sm">No audience roles set</span>
              )}
            </div>
          )}
        </div>

        {/* Other Tags */}
        {otherTags.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">Other Tags</h2>
            <div className="flex flex-wrap gap-2">
              {otherTags.map((tag) => (
                <span
                  key={tag.id}
                  className="px-3 py-1 text-sm rounded-full font-medium bg-stone-100 text-stone-800"
                >
                  <span className="text-stone-500">{tag.kind}:</span> {tag.displayName || tag.value}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Source Pages */}
        {post.sourcePages && post.sourcePages.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">
              Source Pages ({post.sourcePages.length})
            </h2>
            <p className="text-sm text-stone-500 mb-4">
              Pages from which this post was extracted.
            </p>
            <div className="space-y-3">
              {post.sourcePages.map((page) => {
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
                          Fetched {formatDate(page.fetchedAt)}
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
              {post.descriptionMarkdown || post.description}
            </ReactMarkdown>
          </div>
        </div>
      </div>

      {/* Reject Modal */}
      {showRejectModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg p-6 max-w-md w-full">
            <h3 className="text-lg font-semibold text-stone-900 mb-4">Reject Post</h3>
            <p className="text-sm text-stone-600 mb-4">
              Are you sure you want to reject &quot;{post.title}&quot;? You can optionally provide a reason.
            </p>
            <textarea
              value={rejectReason}
              onChange={(e) => setRejectReason(e.target.value)}
              placeholder="Reason for rejection (optional)"
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500 mb-4"
              rows={3}
            />
            <div className="flex gap-2">
              <button
                onClick={handleReject}
                className="flex-1 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
              >
                Reject
              </button>
              <button
                onClick={() => {
                  setShowRejectModal(false);
                  setRejectReason("");
                }}
                className="flex-1 px-4 py-2 bg-stone-200 text-stone-700 rounded hover:bg-stone-300"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
