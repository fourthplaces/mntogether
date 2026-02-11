"use client";

import Link from "next/link";
import { useState, useRef, useEffect } from "react";
import { useParams, useRouter } from "next/navigation";
import { useRestate, callService, callObject, invalidateService, invalidateObject } from "@/lib/restate/client";
import { AdminLoader } from "@/components/admin/AdminLoader";
import type {
  OrganizationResult,
  SourceListResult,
  NoteListResult,
  NoteResult,
  PostList,
  PostResult,
} from "@/lib/restate/types";

const PLATFORMS = ["instagram", "facebook", "tiktok", "x"];

const SOURCE_TYPE_LABELS: Record<string, string> = {
  website: "Website",
  instagram: "Instagram",
  facebook: "Facebook",
  tiktok: "TikTok",
  x: "X (Twitter)",
};

export default function OrganizationDetailPage() {
  const params = useParams();
  const router = useRouter();
  const orgId = params.id as string;

  const [editing, setEditing] = useState(false);
  const [editName, setEditName] = useState("");
  const [editDescription, setEditDescription] = useState("");
  const [editLoading, setEditLoading] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [regenerating, setRegenerating] = useState(false);
  const [regeneratingPosts, setRegeneratingPosts] = useState(false);
  const [generatingNotes, setGeneratingNotes] = useState(false);
  const [extractingOrgPosts, setExtractingOrgPosts] = useState(false);
  const [autoAttaching, setAutoAttaching] = useState(false);
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [rejectReason, setRejectReason] = useState("");
  const [showRejectDialog, setShowRejectDialog] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const {
    data: org,
    isLoading: orgLoading,
    error: orgError,
    mutate: refetchOrg,
  } = useRestate<OrganizationResult>("Organizations", "get", { id: orgId }, {
    revalidateOnFocus: false,
  });

  const { data: sourcesData, mutate: refetchSources } =
    useRestate<SourceListResult>("Sources", "list_by_organization", {
      organization_id: orgId,
    }, { revalidateOnFocus: false });

  const { data: postsData, mutate: refetchPosts } =
    useRestate<PostList>("Posts", "list_by_organization", {
      organization_id: orgId,
    }, { revalidateOnFocus: false });

  const { data: notesData, mutate: refetchNotes } =
    useRestate<NoteListResult>("Notes", "list_for_entity", {
      noteable_type: "organization",
      noteable_id: orgId,
    }, { revalidateOnFocus: false });

  const sources = sourcesData?.sources || [];
  const posts = postsData?.posts || [];
  const notes = notesData?.notes || [];

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
      await callService("Organizations", "update", {
        id: orgId,
        name: editName.trim(),
        description: editDescription.trim() || null,
      });
      invalidateService("Organizations");
      refetchOrg();
      setEditing(false);
    } catch (err: any) {
      setEditError(err.message || "Failed to update organization");
    } finally {
      setEditLoading(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm("Are you sure you want to delete this organization?")) return;
    try {
      await callService("Organizations", "delete", { id: orgId });
      invalidateService("Organizations");
      router.push("/admin/organizations");
    } catch (err: any) {
      alert(err.message || "Failed to delete organization");
    }
  };

  const handleRegenerate = async () => {
    setMenuOpen(false);
    setRegenerating(true);
    try {
      const result = await callService<{ organization_id: string | null; status: string }>(
        "Organizations", "regenerate", { id: orgId }
      );
      invalidateService("Organizations");
      invalidateService("Sources");
      if (result.organization_id && result.organization_id !== orgId) {
        router.push(`/admin/organizations/${result.organization_id}`);
      } else {
        refetchOrg();
        refetchSources();
      }
    } catch (err: any) {
      console.error("Failed to regenerate:", err);
      alert(err.message || "Failed to regenerate organization");
    } finally {
      setRegenerating(false);
    }
  };

  const handleGenerateNotes = async () => {
    setMenuOpen(false);
    setGeneratingNotes(true);
    try {
      await callService<{ notes_created: number; sources_scanned: number; posts_attached: number }>(
        "Notes", "generate_notes", { organization_id: orgId }
      );
      invalidateService("Notes");
      refetchNotes();
    } catch (err: any) {
      console.error("Failed to generate notes:", err);
      alert(err.message || "Failed to generate notes");
    } finally {
      setGeneratingNotes(false);
    }
  };

  const handleAutoAttach = async () => {
    setAutoAttaching(true);
    try {
      await callService<{ notes_count: number; posts_count: number; noteables_created: number }>(
        "Notes", "attach_notes", { organization_id: orgId }
      );
      invalidateService("Notes");
      refetchNotes();
    } catch (err: any) {
      console.error("Failed to auto-attach notes:", err);
      alert(err.message || "Failed to auto-attach notes");
    } finally {
      setAutoAttaching(false);
    }
  };

  const handleRegeneratePosts = async () => {
    setMenuOpen(false);
    setRegeneratingPosts(true);
    try {
      await Promise.all(
        sources.map((source) =>
          callObject("Source", source.id, "regenerate_posts", {})
        )
      );
      invalidateService("Posts");
      invalidateService("Sources");
      sources.forEach((source) => invalidateObject("Source", source.id));
    } catch (err: any) {
      console.error("Failed to regenerate posts:", err);
      alert(err.message || "Failed to regenerate posts");
    } finally {
      setRegeneratingPosts(false);
    }
  };

  const handleExtractOrgPosts = async () => {
    setMenuOpen(false);
    setExtractingOrgPosts(true);
    try {
      await callService("Organizations", "extract_org_posts", { id: orgId });
      invalidateService("Posts");
      invalidateService("Organizations");
      refetchPosts();
      refetchOrg();
    } catch (err: any) {
      console.error("Failed to extract org posts:", err);
      alert(err.message || "Failed to extract org posts");
    } finally {
      setExtractingOrgPosts(false);
    }
  };

  const handleApprove = async () => {
    setActionInProgress("approve");
    try {
      await callService("Organizations", "approve", { id: orgId });
      invalidateService("Organizations");
      refetchOrg();
    } catch (err: any) {
      alert(err.message || "Failed to approve organization");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    if (!rejectReason.trim()) return;
    setActionInProgress("reject");
    try {
      await callService("Organizations", "reject", { id: orgId, reason: rejectReason.trim() });
      invalidateService("Organizations");
      refetchOrg();
      setShowRejectDialog(false);
      setRejectReason("");
    } catch (err: any) {
      alert(err.message || "Failed to reject organization");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleSuspend = async () => {
    const reason = prompt("Suspension reason:");
    if (!reason) return;
    setActionInProgress("suspend");
    try {
      await callService("Organizations", "suspend", { id: orgId, reason });
      invalidateService("Organizations");
      refetchOrg();
    } catch (err: any) {
      alert(err.message || "Failed to suspend organization");
    } finally {
      setActionInProgress(null);
    }
  };

  if (orgLoading) {
    return <AdminLoader label="Loading organization..." />;
  }

  if (orgError) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-5xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-red-600 mb-4">Error</h1>
          <p className="text-stone-600 mb-4">{orgError.message}</p>
          <Link href="/admin/organizations" className="text-amber-600 hover:text-amber-800">
            Back to Organizations
          </Link>
        </div>
      </div>
    );
  }

  if (!org) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-5xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-stone-900 mb-4">Not Found</h1>
          <Link href="/admin/organizations" className="text-amber-600 hover:text-amber-800">
            Back to Organizations
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-5xl mx-auto">
        <Link
          href="/admin/organizations"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Organizations
        </Link>

        {/* Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          {editing ? (
            <form onSubmit={handleUpdate} className="space-y-3">
              <input
                type="text"
                value={editName}
                onChange={(e) => setEditName(e.target.value)}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-lg font-bold focus:outline-none focus:ring-2 focus:ring-amber-500"
                autoFocus
                disabled={editLoading}
              />
              <textarea
                value={editDescription}
                onChange={(e) => setEditDescription(e.target.value)}
                placeholder="Description (optional)"
                rows={3}
                className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                disabled={editLoading}
              />
              <div className="flex items-center gap-2">
                <button
                  type="submit"
                  disabled={editLoading || !editName.trim()}
                  className="px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
                >
                  {editLoading ? "Saving..." : "Save"}
                </button>
                <button
                  type="button"
                  onClick={() => setEditing(false)}
                  className="px-3 py-2 text-stone-500 hover:text-stone-700 text-sm"
                >
                  Cancel
                </button>
                {editError && <span className="text-red-600 text-sm">{editError}</span>}
              </div>
            </form>
          ) : (
            <div className="flex justify-between items-start">
              <div>
                <div className="flex items-center gap-3 mb-1">
                  <h1 className="text-2xl font-bold text-stone-900">{org.name}</h1>
                  <span
                    className={`px-3 py-1 text-sm rounded-full font-medium ${
                      org.status === "approved"
                        ? "bg-green-100 text-green-800"
                        : org.status === "pending_review"
                          ? "bg-yellow-100 text-yellow-800"
                          : org.status === "rejected"
                            ? "bg-red-100 text-red-800"
                            : "bg-gray-100 text-gray-800"
                    }`}
                  >
                    {org.status.replace(/_/g, " ")}
                  </span>
                </div>
                {org.description && (
                  <p className="text-stone-600">{org.description}</p>
                )}
              </div>
              <div className="flex gap-2">
                {org.status === "pending_review" && (
                  <>
                    <button
                      onClick={handleApprove}
                      disabled={actionInProgress !== null}
                      className="px-4 py-1.5 bg-emerald-400 text-white rounded-lg text-sm font-medium hover:bg-emerald-500 disabled:opacity-50 transition-colors"
                    >
                      {actionInProgress === "approve" ? "..." : "Approve"}
                    </button>
                    <button
                      onClick={() => setShowRejectDialog(true)}
                      disabled={actionInProgress !== null}
                      className="px-4 py-1.5 bg-rose-400 text-white rounded-lg text-sm font-medium hover:bg-rose-500 disabled:opacity-50 transition-colors"
                    >
                      Reject
                    </button>
                  </>
                )}
                <button
                  onClick={startEditing}
                  className="px-3 py-1.5 rounded-lg text-sm font-medium bg-stone-100 text-stone-700 hover:bg-stone-200 transition-colors"
                >
                  Edit
                </button>
                <div className="relative" ref={menuRef}>
                  <button
                    onClick={() => setMenuOpen(!menuOpen)}
                    disabled={regenerating || regeneratingPosts || generatingNotes || extractingOrgPosts}
                    className="px-3 py-1.5 bg-stone-100 text-stone-700 rounded-lg hover:bg-stone-200 disabled:opacity-50 text-sm"
                  >
                    {regenerating || regeneratingPosts || generatingNotes || extractingOrgPosts ? "..." : "\u22EF"}
                  </button>
                  {menuOpen && (
                    <div className="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-stone-200 py-1 z-10">
                      <button
                        onClick={handleRegenerate}
                        disabled={regenerating}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        Regenerate with AI
                      </button>
                      <button
                        onClick={handleRegeneratePosts}
                        disabled={regeneratingPosts || sources.length === 0}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {regeneratingPosts ? "Regenerating Posts..." : "Regenerate Posts"}
                      </button>
                      <button
                        onClick={handleGenerateNotes}
                        disabled={generatingNotes}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {generatingNotes ? "Generating Notes..." : "Generate Notes"}
                      </button>
                      <button
                        onClick={handleExtractOrgPosts}
                        disabled={extractingOrgPosts || sources.length === 0}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {extractingOrgPosts ? "Extracting Posts..." : "Extract Org Posts"}
                      </button>
                      {org.status === "approved" && (
                        <button
                          onClick={() => { setMenuOpen(false); handleSuspend(); }}
                          disabled={actionInProgress !== null}
                          className="w-full text-left px-4 py-2 text-sm text-amber-700 hover:bg-amber-50 disabled:opacity-50"
                        >
                          Suspend Organization
                        </button>
                      )}
                      <div className="border-t border-stone-100 my-1" />
                      <button
                        onClick={() => { setMenuOpen(false); handleDelete(); }}
                        className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50"
                      >
                        Delete Organization
                      </button>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {regenerating && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Regenerating organization from website content...
              </span>
            </div>
          )}

          {regeneratingPosts && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Regenerating posts for {sources.length} source{sources.length !== 1 ? "s" : ""}...
              </span>
            </div>
          )}

          {generatingNotes && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Generating notes from crawled content...
              </span>
            </div>
          )}

          {extractingOrgPosts && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Extracting posts across all sources...
              </span>
            </div>
          )}

          <div className="grid grid-cols-4 gap-4 pt-4 mt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Websites</span>
              <p className="text-lg font-semibold text-stone-900">{org.website_count}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Social</span>
              <p className="text-lg font-semibold text-stone-900">{org.social_profile_count}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Posts</span>
              <p className="text-lg font-semibold text-stone-900">{posts.length}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">
                {new Date(org.created_at).toLocaleDateString()}
              </p>
            </div>
          </div>
        </div>

        {/* Sources */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">Sources</h2>
          {sources.length === 0 ? (
            <p className="text-stone-500 text-sm">
              No sources linked. Assign this organization from a source's detail page, or add a social profile below.
            </p>
          ) : (
            <div className="space-y-2">
              {sources.map((source) => (
                <Link
                  key={source.id}
                  href={`/admin/sources/${source.id}`}
                  className="flex items-center justify-between p-3 rounded-lg border border-stone-200 hover:bg-stone-50"
                >
                  <div className="flex items-center gap-3">
                    <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                      source.source_type === "website" ? "bg-blue-100 text-blue-800" :
                      source.source_type === "instagram" ? "bg-purple-100 text-purple-800" :
                      source.source_type === "facebook" ? "bg-indigo-100 text-indigo-800" :
                      source.source_type === "x" ? "bg-stone-800 text-white" :
                      "bg-stone-100 text-stone-800"
                    }`}>
                      {SOURCE_TYPE_LABELS[source.source_type] || source.source_type}
                    </span>
                    <span className="font-medium text-stone-900">{source.identifier}</span>
                    <span
                      className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                        source.status === "approved"
                          ? "bg-green-100 text-green-800"
                          : source.status === "pending_review"
                            ? "bg-yellow-100 text-yellow-800"
                            : "bg-stone-100 text-stone-600"
                      }`}
                    >
                      {source.status.replace(/_/g, " ")}
                    </span>
                  </div>
                  <span className="text-sm text-stone-500">
                    {source.post_count || 0} posts
                  </span>
                </Link>
              ))}
            </div>
          )}

          <div className="mt-4 pt-4 border-t border-stone-200">
            <h3 className="text-sm font-medium text-stone-700 mb-2">Add Social Profile</h3>
            <AddSocialProfileForm orgId={orgId} onAdded={() => {
              refetchSources();
              refetchOrg();
            }} />
          </div>
        </div>

        {/* Posts */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">
            Posts {posts.length > 0 && <span className="text-stone-400 font-normal">({posts.length})</span>}
          </h2>
          {posts.length === 0 ? (
            <p className="text-stone-500 text-sm">No posts found for this organization.</p>
          ) : (
            <div className="space-y-2">
              {posts.map((post) => (
                <div
                  key={post.id}
                  className="flex items-center justify-between p-3 rounded-lg border border-stone-200 hover:bg-stone-50"
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <Link href={`/admin/posts/${post.id}`} className="font-medium text-stone-900 truncate hover:underline">
                      {post.title}
                    </Link>
                    <select
                      value={post.status}
                      onChange={async (e) => {
                        const newStatus = e.target.value;
                        try {
                          if (newStatus === "active") {
                            await callObject("Post", post.id, "approve", {});
                          } else if (newStatus === "rejected") {
                            await callObject("Post", post.id, "reject", { reason: "Rejected by admin" });
                          }
                          invalidateService("Posts");
                          invalidateObject("Post", post.id);
                          refetchPosts();
                        } catch (err: any) {
                          console.error("Failed to update post status:", err);
                        }
                      }}
                      className={`px-2 py-0.5 text-xs rounded-full font-medium border-0 cursor-pointer appearance-none pr-5 shrink-0 ${
                        post.status === "active"
                          ? "bg-green-100 text-green-800"
                          : post.status === "pending_approval"
                            ? "bg-yellow-100 text-yellow-800"
                            : post.status === "rejected"
                              ? "bg-red-100 text-red-800"
                              : "bg-stone-100 text-stone-600"
                      }`}
                      style={{ backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath d='M0 2l4 4 4-4z' fill='%23666'/%3E%3C/svg%3E\")", backgroundRepeat: "no-repeat", backgroundPosition: "right 4px center" }}
                    >
                      <option value="pending_approval">pending</option>
                      <option value="active">active</option>
                      <option value="rejected">rejected</option>
                    </select>
                    <span className={`px-2 py-0.5 text-xs rounded-full font-medium shrink-0 ${
                      post.post_type === "service"
                        ? "bg-blue-100 text-blue-800"
                        : post.post_type === "opportunity"
                          ? "bg-purple-100 text-purple-800"
                          : post.post_type === "business"
                            ? "bg-amber-100 text-amber-800"
                            : "bg-stone-100 text-stone-600"
                    }`}>
                      {post.post_type}
                    </span>
                  </div>
                  <span className="text-sm text-stone-500 shrink-0 ml-3">
                    {new Date(post.created_at).toLocaleDateString()}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Notes */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-stone-900">Notes</h2>
            {notes.length > 0 && (
              <button
                onClick={handleAutoAttach}
                disabled={autoAttaching}
                className="px-3 py-1.5 text-xs font-medium text-amber-700 bg-amber-50 rounded-lg hover:bg-amber-100 disabled:opacity-50 transition-colors"
              >
                {autoAttaching ? "Attaching..." : "Auto Attach to Posts"}
              </button>
            )}
          </div>

          <AddNoteForm
            noteableType="organization"
            noteableId={orgId}
            onAdded={() => refetchNotes()}
          />

          {notes.length === 0 ? (
            <p className="text-stone-500 text-sm mt-4">No notes yet.</p>
          ) : (
            <div className="space-y-2 mt-4">
              {notes.map((note) => (
                <NoteRow
                  key={note.id}
                  note={note}
                  noteableType="organization"
                  noteableId={orgId}
                  onChanged={() => refetchNotes()}
                />
              ))}
            </div>
          )}
        </div>

        {/* Reject Dialog */}
        {showRejectDialog && (
          <>
            <div
              className="fixed inset-0 bg-black/40 z-40"
              onClick={() => setShowRejectDialog(false)}
            />
            <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
              <div className="bg-white rounded-xl shadow-xl w-full max-w-md">
                <div className="px-5 py-4 border-b border-stone-200">
                  <h2 className="text-lg font-semibold text-stone-900">Reject Organization</h2>
                </div>
                <div className="p-5 space-y-3">
                  <textarea
                    value={rejectReason}
                    onChange={(e) => setRejectReason(e.target.value)}
                    placeholder="Reason for rejection..."
                    rows={3}
                    className="w-full px-3 py-2 border border-stone-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
                    autoFocus
                  />
                  <div className="flex justify-end gap-2">
                    <button
                      onClick={() => { setShowRejectDialog(false); setRejectReason(""); }}
                      className="px-4 py-2 text-stone-500 hover:text-stone-700 text-sm"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={handleReject}
                      disabled={!rejectReason.trim() || actionInProgress !== null}
                      className="px-4 py-2 bg-rose-500 text-white rounded-lg text-sm font-medium hover:bg-rose-600 disabled:opacity-50 transition-colors"
                    >
                      {actionInProgress === "reject" ? "Rejecting..." : "Reject"}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </>
        )}

      </div>
    </div>
  );
}

function AddSocialProfileForm({
  orgId,
  onAdded,
}: {
  orgId: string;
  onAdded: () => void;
}) {
  const [platform, setPlatform] = useState("instagram");
  const [handle, setHandle] = useState("");
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!handle.trim()) return;

    setLoading(true);
    setError(null);
    try {
      await callService("Sources", "create_social", {
        organization_id: orgId,
        platform: platform,
        handle: handle.trim(),
        url: url.trim() || null,
      });
      invalidateService("Sources");
      setHandle("");
      setUrl("");
      onAdded();
    } catch (err: any) {
      setError(err.message || "Failed to add profile");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex items-center gap-2 bg-stone-50 rounded-lg px-3 py-2"
    >
      <select
        value={platform}
        onChange={(e) => setPlatform(e.target.value)}
        className="px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 bg-white"
        disabled={loading}
      >
        {PLATFORMS.map((p) => (
          <option key={p} value={p}>
            {p.charAt(0).toUpperCase() + p.slice(1)}
          </option>
        ))}
      </select>
      <input
        type="text"
        value={handle}
        onChange={(e) => setHandle(e.target.value)}
        placeholder="Handle (e.g. @example)"
        className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <input
        type="text"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="URL (optional)"
        className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      <button
        type="submit"
        disabled={loading || !handle.trim()}
        className="px-3 py-1.5 bg-amber-600 text-white rounded text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
      >
        {loading ? "..." : "Add"}
      </button>
      {error && <span className="text-red-600 text-xs">{error}</span>}
    </form>
  );
}

function AddNoteForm({
  noteableType,
  noteableId,
  onAdded,
}: {
  noteableType: string;
  noteableId: string;
  onAdded: () => void;
}) {
  const [content, setContent] = useState("");
  const [ctaText, setCtaText] = useState("");
  const [severity, setSeverity] = useState("info");
  const [isPublic, setIsPublic] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim()) return;

    setLoading(true);
    setError(null);
    try {
      await callService("Notes", "create", {
        content: content.trim(),
        severity,
        is_public: isPublic,
        cta_text: ctaText.trim() || null,
        noteable_type: noteableType,
        noteable_id: noteableId,
      });
      invalidateService("Notes");
      setContent("");
      setCtaText("");
      setSeverity("info");
      setIsPublic(false);
      onAdded();
    } catch (err: any) {
      setError(err.message || "Failed to add note");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-2 bg-stone-50 rounded-lg px-3 py-2">
      <div className="flex items-center gap-2">
        <select
          value={severity}
          onChange={(e) => setSeverity(e.target.value)}
          className="px-2 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 bg-white"
          disabled={loading}
        >
          <option value="info">Info</option>
          <option value="notice">Notice</option>
          <option value="urgent">Urgent</option>
        </select>
        <input
          type="text"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Add a note..."
          className="flex-1 px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
          disabled={loading}
        />
        <label className="flex items-center gap-1 text-xs text-stone-500 cursor-pointer">
          <input
            type="checkbox"
            checked={isPublic}
            onChange={(e) => setIsPublic(e.target.checked)}
            className="rounded border-stone-300"
            disabled={loading}
          />
          Public
        </label>
        <button
          type="submit"
          disabled={loading || !content.trim()}
          className="px-3 py-1.5 bg-amber-600 text-white rounded text-sm font-medium hover:bg-amber-700 disabled:opacity-50 transition-colors"
        >
          {loading ? "..." : "Add"}
        </button>
      </div>
      <input
        type="text"
        value={ctaText}
        onChange={(e) => setCtaText(e.target.value)}
        placeholder="Call to action (optional)"
        className="w-full px-3 py-1.5 border border-stone-300 rounded text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
        disabled={loading}
      />
      {error && <span className="text-red-600 text-xs">{error}</span>}
    </form>
  );
}

const SEVERITY_STYLES: Record<string, string> = {
  urgent: "bg-red-100 text-red-800",
  notice: "bg-yellow-100 text-yellow-800",
  info: "bg-blue-100 text-blue-800",
};

const SEVERITY_OPTIONS = ["info", "notice", "urgent"] as const;

function NoteRow({
  note,
  noteableType,
  noteableId,
  onChanged,
}: {
  note: NoteResult;
  noteableType: string;
  noteableId: string;
  onChanged: () => void;
}) {
  const isExpired = !!note.expired_at;

  const handleTogglePublic = async () => {
    try {
      await callService("Notes", "update", {
        id: note.id,
        content: note.content,
        severity: note.severity,
        is_public: !note.is_public,
        cta_text: note.cta_text,
      });
      invalidateService("Notes");
      onChanged();
    } catch (err: any) {
      console.error("Failed to toggle visibility:", err);
    }
  };

  const handleSeverityChange = async (newSeverity: string) => {
    try {
      await callService("Notes", "update", {
        id: note.id,
        content: note.content,
        severity: newSeverity,
        is_public: note.is_public,
        cta_text: note.cta_text,
      });
      invalidateService("Notes");
      onChanged();
    } catch (err: any) {
      console.error("Failed to update severity:", err);
    }
  };

  const handleDelete = async () => {
    try {
      await callService("Notes", "delete", { id: note.id });
      invalidateService("Notes");
      onChanged();
    } catch (err: any) {
      console.error("Failed to delete note:", err);
    }
  };

  const handleUnlink = async () => {
    try {
      await callService("Notes", "unlink", {
        note_id: note.id,
        noteable_type: noteableType,
        noteable_id: noteableId,
      });
      invalidateService("Notes");
      onChanged();
    } catch (err: any) {
      console.error("Failed to unlink note:", err);
    }
  };

  return (
    <div
      className={`flex items-start justify-between p-3 rounded-lg border ${
        isExpired ? "border-stone-200 bg-stone-50 opacity-60" : "border-stone-200"
      }`}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <select
            value={note.severity}
            onChange={(e) => handleSeverityChange(e.target.value)}
            className={`px-2 py-0.5 text-xs rounded-full font-medium border-0 cursor-pointer appearance-none pr-5 ${
              SEVERITY_STYLES[note.severity] || SEVERITY_STYLES.info
            }`}
            style={{ backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath d='M0 2l4 4 4-4z' fill='%23666'/%3E%3C/svg%3E\")", backgroundRepeat: "no-repeat", backgroundPosition: "right 4px center" }}
          >
            {SEVERITY_OPTIONS.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </select>
          <button
            onClick={handleTogglePublic}
            className={`px-2 py-0.5 text-xs rounded-full font-medium cursor-pointer transition-colors ${
              note.is_public
                ? "bg-green-100 text-green-800 hover:bg-green-200"
                : "bg-stone-100 text-stone-500 hover:bg-stone-200"
            }`}
          >
            {note.is_public ? "public" : "internal"}
          </button>
          {isExpired && (
            <span className="px-2 py-0.5 text-xs rounded-full font-medium bg-stone-200 text-stone-600">
              expired
            </span>
          )}
          {note.source_type && (
            <span className="text-xs text-stone-400">
              via {note.source_type}
            </span>
          )}
          <span className="text-xs text-stone-400">
            {note.created_by} &middot; {new Date(note.created_at).toLocaleDateString()}
          </span>
        </div>
        <p className="text-sm text-stone-700">{note.content}</p>
        {note.cta_text && (
          <p className="text-xs italic text-stone-500 mt-0.5">{note.cta_text}</p>
        )}
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
        {note.linked_posts && note.linked_posts.length > 0 && (
          <div className="flex flex-wrap items-center gap-1 mt-1.5">
            <span className="text-xs text-stone-400">Attached to:</span>
            {note.linked_posts.map((post) => (
              <Link
                key={post.id}
                href={`/admin/posts/${post.id}`}
                className="text-xs px-1.5 py-0.5 bg-stone-100 text-stone-600 rounded hover:bg-stone-200 hover:text-stone-800 transition-colors truncate max-w-[200px]"
                title={post.title}
              >
                {post.title}
              </Link>
            ))}
          </div>
        )}
      </div>
      <div className="flex gap-1 ml-2 shrink-0">
        <button
          onClick={handleUnlink}
          className="px-2 py-1 text-xs text-stone-500 hover:text-amber-700 hover:bg-amber-50 rounded transition-colors"
          title="Unlink from this entity"
        >
          Unlink
        </button>
        <button
          onClick={handleDelete}
          className="px-2 py-1 text-xs text-stone-500 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
          title="Delete note entirely"
        >
          Delete
        </button>
      </div>
    </div>
  );
}

