"use client";

import Link from "next/link";
import { useState, useRef, useEffect } from "react";
import { useParams, useRouter } from "next/navigation";
import { useQuery, useMutation } from "urql";
import { AdminLoader } from "@/components/admin/AdminLoader";
import {
  OrganizationDetailQuery,
  OrganizationChecklistQuery,
  UpdateOrganizationMutation,
  DeleteOrganizationMutation,
  ApproveOrganizationMutation,
  RejectOrganizationMutation,
  SuspendOrganizationMutation,
  SetOrganizationStatusMutation,
  ToggleChecklistItemMutation,
  RegenerateOrganizationMutation,
  ExtractOrgPostsMutation,
  CleanUpOrgPostsMutation,
  RunCuratorMutation,
  RemoveAllOrgPostsMutation,
  RemoveAllOrgNotesMutation,
  RewriteNarrativesMutation,
} from "@/lib/graphql/organizations";
import {
  OrganizationSourcesQuery,
  OrganizationPostsQuery,
  EntityNotesQuery,
  CreateNoteMutation,
  UpdateNoteMutation,
  DeleteNoteMutation,
  UnlinkNoteMutation,
  GenerateNotesFromSourcesMutation,
  AutoAttachNotesMutation,
  CreateSocialSourceMutation,
  CrawlAllOrgSourcesMutation,
} from "@/lib/graphql/notes";
import {
  RegenerateSourcePostsMutation,
} from "@/lib/graphql/sources";
import {
  ApprovePostMutation,
  RejectPostMutation,
  ReactivatePostMutation,
  UpdatePostCapacityMutation,
  BatchScorePostsMutation,
} from "@/lib/graphql/posts";

const PLATFORMS = ["instagram", "facebook", "tiktok", "x"];

const SOURCE_TYPE_LABELS: Record<string, string> = {
  website: "Website",
  instagram: "Instagram",
  facebook: "Facebook",
  tiktok: "TikTok",
  x: "X (Twitter)",
};

const orgMutationContext = { additionalTypenames: ["Organization", "Checklist"] };
const postMutationContext = { additionalTypenames: ["Post", "PostConnection"] };
const noteMutationContext = { additionalTypenames: ["Note"] };
const sourceMutationContext = { additionalTypenames: ["Source", "SourceConnection"] };

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
  const [cleaningUpPosts, setCleaningUpPosts] = useState(false);
  const [runningCurator, setRunningCurator] = useState(false);
  const [crawlingAll, setCrawlingAll] = useState(false);
  const [autoAttaching, setAutoAttaching] = useState(false);
  const [removingAllPosts, setRemovingAllPosts] = useState(false);
  const [removingAllNotes, setRemovingAllNotes] = useState(false);
  const [rewritingNarratives, setRewritingNarratives] = useState(false);
  const [rewriteResult, setRewriteResult] = useState<string | null>(null);
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

  // --- Data queries ---

  const [{ data: orgData, fetching: orgLoading, error: orgError }] = useQuery({
    query: OrganizationDetailQuery,
    variables: { id: orgId },
  });

  const org = orgData?.organization;

  const [{ data: sourcesData }] = useQuery({
    query: OrganizationSourcesQuery,
    variables: { organizationId: orgId },
  });

  const [{ data: postsData }] = useQuery({
    query: OrganizationPostsQuery,
    variables: { organizationId: orgId },
  });

  const [{ data: notesData }] = useQuery({
    query: EntityNotesQuery,
    variables: { noteableType: "organization", noteableId: orgId },
  });

  const [{ data: checklistData }] = useQuery({
    query: OrganizationChecklistQuery,
    variables: { id: orgId },
  });

  const sources = sourcesData?.organizationSources || [];
  const posts = postsData?.organizationPosts?.posts || [];
  const notes = notesData?.entityNotes || [];
  const checklist = checklistData?.organizationChecklist;

  // --- Mutations ---

  const [, updateOrg] = useMutation(UpdateOrganizationMutation);
  const [, deleteOrg] = useMutation(DeleteOrganizationMutation);
  const [, approveOrg] = useMutation(ApproveOrganizationMutation);
  const [, rejectOrg] = useMutation(RejectOrganizationMutation);
  const [, suspendOrg] = useMutation(SuspendOrganizationMutation);
  const [, setOrgStatus] = useMutation(SetOrganizationStatusMutation);
  const [, toggleChecklistItem] = useMutation(ToggleChecklistItemMutation);
  const [, regenerateOrg] = useMutation(RegenerateOrganizationMutation);
  const [, extractOrgPosts] = useMutation(ExtractOrgPostsMutation);
  const [, cleanUpOrgPosts] = useMutation(CleanUpOrgPostsMutation);
  const [, runCurator] = useMutation(RunCuratorMutation);
  const [, removeAllOrgPosts] = useMutation(RemoveAllOrgPostsMutation);
  const [, removeAllOrgNotes] = useMutation(RemoveAllOrgNotesMutation);
  const [, rewriteNarratives] = useMutation(RewriteNarrativesMutation);
  const [, generateNotesFromSources] = useMutation(GenerateNotesFromSourcesMutation);
  const [, autoAttachNotes] = useMutation(AutoAttachNotesMutation);
  const [, regenerateSourcePosts] = useMutation(RegenerateSourcePostsMutation);
  const [, crawlAllOrgSources] = useMutation(CrawlAllOrgSourcesMutation);

  // --- Actions ---

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

  const handleDelete = async () => {
    if (!confirm("Are you sure you want to delete this organization?")) return;
    try {
      const result = await deleteOrg({ id: orgId }, orgMutationContext);
      if (result.error) throw result.error;
      router.push("/admin/organizations");
    } catch (err: any) {
      alert(err.message || "Failed to delete organization");
    }
  };

  const handleRegenerate = async () => {
    setMenuOpen(false);
    setRegenerating(true);
    try {
      const result = await regenerateOrg({ id: orgId }, orgMutationContext);
      if (result.error) throw result.error;
      const data = result.data?.regenerateOrganization;
      if (data?.organizationId && data.organizationId !== orgId) {
        router.push(`/admin/organizations/${data.organizationId}`);
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
      const result = await generateNotesFromSources(
        { organizationId: orgId },
        noteMutationContext,
      );
      if (result.error) throw result.error;
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
      const result = await autoAttachNotes(
        { organizationId: orgId },
        noteMutationContext,
      );
      if (result.error) throw result.error;
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
          regenerateSourcePosts({ id: source.id }, sourceMutationContext)
        )
      );
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
      const result = await extractOrgPosts(
        { id: orgId },
        { additionalTypenames: ["Post", "PostConnection", "Organization"] },
      );
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to extract org posts:", err);
      alert(err.message || "Failed to extract org posts");
    } finally {
      setExtractingOrgPosts(false);
    }
  };

  const handleCleanUpPosts = async () => {
    setMenuOpen(false);
    setCleaningUpPosts(true);
    try {
      const result = await cleanUpOrgPosts({ id: orgId }, postMutationContext);
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to clean up posts:", err);
      alert(err.message || "Failed to clean up posts");
    } finally {
      setCleaningUpPosts(false);
    }
  };

  const handleRunCurator = async () => {
    setMenuOpen(false);
    setRunningCurator(true);
    try {
      const result = await runCurator({ id: orgId }, orgMutationContext);
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to run curator:", err);
      alert(err.message || "Failed to run curator");
    } finally {
      setRunningCurator(false);
    }
  };

  const handleCrawlAll = async () => {
    setMenuOpen(false);
    setCrawlingAll(true);
    try {
      const result = await crawlAllOrgSources(
        { organizationId: orgId },
        sourceMutationContext,
      );
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to crawl sources:", err);
      alert(err.message || "Failed to crawl sources");
    } finally {
      setCrawlingAll(false);
    }
  };

  const handleRemoveAllPosts = async () => {
    if (!confirm(`Are you sure you want to delete ALL posts for "${org?.name}"? This cannot be undone.`)) return;
    setMenuOpen(false);
    setRemovingAllPosts(true);
    try {
      const result = await removeAllOrgPosts({ id: orgId }, postMutationContext);
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to remove posts:", err);
      alert(err.message || "Failed to remove posts");
    } finally {
      setRemovingAllPosts(false);
    }
  };

  const handleRemoveAllNotes = async () => {
    if (!confirm(`Are you sure you want to delete ALL notes for "${org?.name}"? This cannot be undone.`)) return;
    setMenuOpen(false);
    setRemovingAllNotes(true);
    try {
      const result = await removeAllOrgNotes({ id: orgId }, noteMutationContext);
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to remove notes:", err);
      alert(err.message || "Failed to remove notes");
    } finally {
      setRemovingAllNotes(false);
    }
  };

  const handleRewriteNarratives = async () => {
    setMenuOpen(false);
    setRewritingNarratives(true);
    setRewriteResult(null);
    try {
      const result = await rewriteNarratives(
        { organizationId: orgId },
        postMutationContext,
      );
      if (result.error) throw result.error;
      const data = result.data?.rewriteNarratives;
      if (data) {
        setRewriteResult(`Rewrote ${data.rewritten} of ${data.total} posts${data.failed > 0 ? ` (${data.failed} failed)` : ""}`);
      }
    } catch (err: any) {
      console.error("Failed to rewrite narratives:", err);
      setRewriteResult(`Error: ${err.message || "Failed to rewrite narratives"}`);
    } finally {
      setRewritingNarratives(false);
    }
  };

  const handleApprove = async () => {
    setActionInProgress("approve");
    try {
      const result = await approveOrg({ id: orgId }, orgMutationContext);
      if (result.error) throw result.error;
    } catch (err: any) {
      alert(err.message || "Failed to approve organization");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleStatusChange = async (newStatus: string) => {
    if (!org || newStatus === org.status) return;

    // Confirm destructive transitions
    if (newStatus === "rejected" || newStatus === "suspended") {
      const reason = prompt(`Reason for ${newStatus === "rejected" ? "rejection" : "suspension"}:`);
      if (!reason) return;
      setActionInProgress("status");
      try {
        const result = await setOrgStatus(
          { id: orgId, status: newStatus, reason },
          orgMutationContext,
        );
        if (result.error) throw result.error;
      } catch (err: any) {
        alert(err.message || `Failed to change status to ${newStatus}`);
      } finally {
        setActionInProgress(null);
      }
      return;
    }

    // For approved, check that checklist is complete
    if (newStatus === "approved" && !checklist?.allChecked) {
      alert("Complete the pre-launch checklist before approving.");
      return;
    }

    setActionInProgress("status");
    try {
      const result = await setOrgStatus(
        { id: orgId, status: newStatus },
        orgMutationContext,
      );
      if (result.error) throw result.error;
    } catch (err: any) {
      alert(err.message || `Failed to change status to ${newStatus}`);
    } finally {
      setActionInProgress(null);
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
      const result = await suspendOrg(
        { id: orgId, reason },
        orgMutationContext,
      );
      if (result.error) throw result.error;
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
                  <select
                    value={org.status}
                    onChange={(e) => handleStatusChange(e.target.value)}
                    disabled={actionInProgress !== null}
                    className={`px-3 py-1 text-sm rounded-full font-medium border-0 cursor-pointer appearance-none pr-6 ${
                      org.status === "approved"
                        ? "bg-green-100 text-green-800"
                        : org.status === "pending_review"
                          ? "bg-yellow-100 text-yellow-800"
                          : org.status === "rejected"
                            ? "bg-red-100 text-red-800"
                            : "bg-gray-100 text-gray-800"
                    }`}
                    style={{ backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath d='M0 2l4 4 4-4z' fill='%23666'/%3E%3C/svg%3E\")", backgroundRepeat: "no-repeat", backgroundPosition: "right 6px center" }}
                  >
                    <option value="pending_review">pending review</option>
                    <option value="approved">approved</option>
                    <option value="rejected">rejected</option>
                    <option value="suspended">suspended</option>
                  </select>
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
                      disabled={actionInProgress !== null || !checklist?.allChecked}
                      title={!checklist?.allChecked ? "Complete the pre-launch checklist first" : undefined}
                      className="px-4 py-1.5 bg-emerald-400 text-white rounded-lg text-sm font-medium hover:bg-emerald-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
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
                    disabled={regenerating || regeneratingPosts || generatingNotes || extractingOrgPosts || cleaningUpPosts || crawlingAll || rewritingNarratives || runningCurator || removingAllPosts || removingAllNotes}
                    className="px-3 py-1.5 bg-stone-100 text-stone-700 rounded-lg hover:bg-stone-200 disabled:opacity-50 text-sm"
                  >
                    {regenerating || regeneratingPosts || generatingNotes || extractingOrgPosts || cleaningUpPosts || crawlingAll || rewritingNarratives || runningCurator || removingAllPosts || removingAllNotes ? "..." : "\u22EF"}
                  </button>
                  {menuOpen && (
                    <div className="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-stone-200 py-1 z-10">
                      <button
                        onClick={handleCrawlAll}
                        disabled={crawlingAll || sources.length === 0}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {crawlingAll ? "Crawling..." : "Crawl All Sources"}
                      </button>
                      <div className="border-t border-stone-100 my-1" />
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
                      <button
                        onClick={handleRunCurator}
                        disabled={runningCurator || sources.length === 0}
                        className="w-full text-left px-4 py-2 text-sm text-amber-700 hover:bg-amber-50 disabled:opacity-50 font-medium"
                      >
                        {runningCurator ? "Curating..." : "Curate"}
                      </button>
                      <button
                        onClick={handleCleanUpPosts}
                        disabled={cleaningUpPosts}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {cleaningUpPosts ? "Cleaning Up..." : "Clean Up Posts"}
                      </button>
                      <button
                        onClick={handleRewriteNarratives}
                        disabled={rewritingNarratives || posts.length === 0}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {rewritingNarratives ? "Rewriting..." : "Rewrite Narratives"}
                      </button>
                      <div className="border-t border-stone-100 my-1" />
                      <button
                        onClick={handleRemoveAllPosts}
                        disabled={removingAllPosts || posts.length === 0}
                        className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
                      >
                        {removingAllPosts ? "Removing Posts..." : "Remove All Posts"}
                      </button>
                      <button
                        onClick={handleRemoveAllNotes}
                        disabled={removingAllNotes || notes.length === 0}
                        className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
                      >
                        {removingAllNotes ? "Removing Notes..." : "Remove All Notes"}
                      </button>
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

          {crawlingAll && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Crawling {sources.length} source{sources.length !== 1 ? "s" : ""}...
              </span>
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

          {runningCurator && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Running curator...
              </span>
            </div>
          )}

          {removingAllPosts && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-red-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-red-700">
                Removing all posts...
              </span>
            </div>
          )}

          {removingAllNotes && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-red-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-red-700">
                Removing all notes...
              </span>
            </div>
          )}

          {cleaningUpPosts && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Cleaning up duplicate and rejected posts...
              </span>
            </div>
          )}

          {rewritingNarratives && (
            <div className="flex items-center gap-3 mt-4 pt-4 border-t border-stone-200">
              <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
              <span className="text-sm font-medium text-amber-700">
                Rewriting titles and summaries...
              </span>
            </div>
          )}

          {rewriteResult && (
            <div className={`mt-4 pt-4 border-t border-stone-200 text-sm font-medium ${rewriteResult.startsWith("Error") ? "text-red-600" : "text-green-700"}`}>
              {rewriteResult}
            </div>
          )}

          <div className="grid grid-cols-4 gap-4 pt-4 mt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Websites</span>
              <p className="text-lg font-semibold text-stone-900">{org.websiteCount}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Social</span>
              <p className="text-lg font-semibold text-stone-900">{org.socialProfileCount}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Snapshots</span>
              <p className="text-lg font-semibold text-stone-900">{org.snapshotCount}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">
                {new Date(org.createdAt).toLocaleDateString()}
              </p>
            </div>
          </div>

          {org.status === "pending_review" && checklist && (
            <div className="pt-4 mt-4 border-t border-stone-200">
              <h3 className="text-sm font-semibold text-stone-700 mb-3">Pre-Launch Checklist</h3>
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
                      className="h-4 w-4 rounded border-stone-300 text-amber-600 focus:ring-amber-500 cursor-pointer"
                    />
                    <span className={`text-sm ${item.checked ? "text-stone-500 line-through" : "text-stone-700"}`}>
                      {item.label}
                    </span>
                    {item.checked && item.checkedAt && (
                      <span className="text-xs text-stone-400">
                        {new Date(item.checkedAt).toLocaleDateString()}
                      </span>
                    )}
                  </label>
                ))}
              </div>
              {!checklist.allChecked && (
                <p className="text-xs text-amber-600 mt-3">
                  Complete all items before approving this organization.
                </p>
              )}
            </div>
          )}
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
                      source.sourceType === "website" ? "bg-blue-100 text-blue-800" :
                      source.sourceType === "instagram" ? "bg-purple-100 text-purple-800" :
                      source.sourceType === "facebook" ? "bg-indigo-100 text-indigo-800" :
                      source.sourceType === "x" ? "bg-stone-800 text-white" :
                      "bg-stone-100 text-stone-800"
                    }`}>
                      {SOURCE_TYPE_LABELS[source.sourceType] || source.sourceType}
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
                    {source.snapshotCount || 0} snapshots
                  </span>
                </Link>
              ))}
            </div>
          )}

          <div className="mt-4 pt-4 border-t border-stone-200">
            <h3 className="text-sm font-medium text-stone-700 mb-2">Add Social Profile</h3>
            <AddSocialProfileForm orgId={orgId} />
          </div>
        </div>

        {/* Posts */}
        <PostsSection
          posts={posts}
          organizationId={orgId}
        />

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

type PostStatusTab = "pending_approval" | "active" | "rejected";

const POST_STATUS_TABS: { value: PostStatusTab; label: string }[] = [
  { value: "pending_approval", label: "Pending" },
  { value: "active", label: "Active" },
  { value: "rejected", label: "Rejected" },
];

type PostData = {
  id: string;
  title: string;
  status: string;
  postType?: string | null;
  capacityStatus?: string | null;
  relevanceScore?: number | null;
  createdAt: string;
};

function PostsSection({
  posts,
  organizationId,
}: {
  posts: PostData[];
  organizationId: string;
}) {
  const [tab, setTab] = useState<PostStatusTab>("pending_approval");
  const [scoring, setScoring] = useState(false);
  const [scoreResult, setScoreResult] = useState<string | null>(null);

  const [, batchScorePosts] = useMutation(BatchScorePostsMutation);

  const handleScorePosts = async () => {
    setScoring(true);
    setScoreResult(null);
    try {
      const result = await batchScorePosts({ limit: 200 }, postMutationContext);
      if (result.error) throw result.error;
      const data = result.data?.batchScorePosts;
      if (data) {
        setScoreResult(`Scored ${data.scored} posts${data.failed > 0 ? `, ${data.failed} failed` : ""}${data.remaining > 0 ? `, ${data.remaining} remaining` : ""}`);
      }
    } catch (err: any) {
      setScoreResult(`Error: ${err.message || "Failed to score posts"}`);
    } finally {
      setScoring(false);
    }
  };

  const unscoredCount = posts.filter((p) => p.status === "active" && p.relevanceScore == null).length;

  const counts = {
    pending_approval: posts.filter((p) => p.status === "pending_approval").length,
    active: posts.filter((p) => p.status === "active").length,
    rejected: posts.filter((p) => p.status === "rejected").length,
  };

  const filtered = posts.filter((p) => p.status === tab);

  return (
    <div className="bg-white rounded-lg shadow-md p-6 mb-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-stone-900">
          Posts {posts.length > 0 && <span className="text-stone-400 font-normal">({posts.length})</span>}
        </h2>
        {unscoredCount > 0 && (
          <button
            onClick={handleScorePosts}
            disabled={scoring}
            className="px-3 py-1.5 text-xs font-medium rounded-lg bg-indigo-50 text-indigo-700 hover:bg-indigo-100 disabled:opacity-50 transition-colors"
          >
            {scoring ? "Scoring..." : `Score ${unscoredCount} Unscored`}
          </button>
        )}
      </div>
      {scoreResult && (
        <div className={`mb-3 px-3 py-2 rounded text-xs ${scoreResult.startsWith("Error") ? "bg-red-50 text-red-700" : "bg-green-50 text-green-700"}`}>
          {scoreResult}
        </div>
      )}
      <div className="flex gap-1 mb-4">
        {POST_STATUS_TABS.map((t) => (
          <button
            key={t.value}
            onClick={() => setTab(t.value)}
            className={`px-3 py-1.5 text-sm font-medium rounded ${
              tab === t.value
                ? t.value === "active"
                  ? "bg-green-100 text-green-800"
                  : t.value === "rejected"
                    ? "bg-red-100 text-red-800"
                    : "bg-yellow-100 text-yellow-800"
                : "text-stone-600 hover:bg-stone-100"
            }`}
          >
            {t.label}
            {counts[t.value] > 0 && (
              <span className="ml-1.5 text-xs opacity-70">{counts[t.value]}</span>
            )}
          </button>
        ))}
      </div>
      {filtered.length === 0 ? (
        <p className="text-stone-500 text-sm">No {tab === "pending_approval" ? "pending" : tab} posts.</p>
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

function PostRow({
  post,
}: {
  post: PostData;
}) {
  const [status, setStatus] = useState(post.status);
  const [capacityStatus, setCapacityStatus] = useState(post.capacityStatus || "accepting");
  const [loading, setLoading] = useState(false);

  const [, approvePost] = useMutation(ApprovePostMutation);
  const [, rejectPost] = useMutation(RejectPostMutation);
  const [, reactivatePost] = useMutation(ReactivatePostMutation);
  const [, updatePostCapacity] = useMutation(UpdatePostCapacityMutation);

  // Sync local state when server data changes
  useEffect(() => {
    setStatus(post.status);
  }, [post.status]);

  useEffect(() => {
    setCapacityStatus(post.capacityStatus || "accepting");
  }, [post.capacityStatus]);

  const handleStatusChange = async (newStatus: string) => {
    if (newStatus === status) return;
    const previousStatus = status;
    setStatus(newStatus);
    setLoading(true);

    try {
      let result;
      if (newStatus === "active") {
        result = await approvePost({ id: post.id }, postMutationContext);
      } else if (newStatus === "rejected") {
        result = await rejectPost({ id: post.id, reason: "Rejected by admin" }, postMutationContext);
      } else if (newStatus === "pending_approval") {
        result = await reactivatePost({ id: post.id }, postMutationContext);
      }
      if (result?.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to update post status:", err);
      setStatus(previousStatus);
      alert(err.message || "Failed to update post status");
    } finally {
      setLoading(false);
    }
  };

  const handleCapacityChange = async (newCapacity: string) => {
    if (newCapacity === capacityStatus) return;
    const previous = capacityStatus;
    setCapacityStatus(newCapacity);
    setLoading(true);

    try {
      const result = await updatePostCapacity(
        { id: post.id, capacityStatus: newCapacity },
        postMutationContext,
      );
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to update capacity status:", err);
      setCapacityStatus(previous);
      alert(err.message || "Failed to update capacity status");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex items-center justify-between p-3 rounded-lg border border-stone-200 hover:bg-stone-50">
      <div className="flex items-center gap-3 min-w-0">
        <Link href={`/admin/posts/${post.id}`} className="font-medium text-stone-900 truncate hover:underline">
          {post.title}
        </Link>
        <select
          value={status}
          disabled={loading}
          onChange={(e) => handleStatusChange(e.target.value)}
          className={`px-2 py-0.5 text-xs rounded-full font-medium border-0 cursor-pointer appearance-none pr-5 shrink-0 ${
            loading ? "opacity-50" :
            status === "active"
              ? "bg-green-100 text-green-800"
              : status === "pending_approval"
                ? "bg-yellow-100 text-yellow-800"
                : status === "rejected"
                  ? "bg-red-100 text-red-800"
                  : "bg-stone-100 text-stone-600"
          }`}
          style={{ backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath d='M0 2l4 4 4-4z' fill='%23666'/%3E%3C/svg%3E\")", backgroundRepeat: "no-repeat", backgroundPosition: "right 4px center" }}
        >
          <option value="pending_approval">pending</option>
          <option value="active">active</option>
          <option value="rejected">rejected</option>
        </select>
        {status === "active" && (
          <select
            value={capacityStatus}
            disabled={loading}
            onChange={(e) => handleCapacityChange(e.target.value)}
            className={`px-2 py-0.5 text-xs rounded-full font-medium border-0 cursor-pointer appearance-none pr-5 shrink-0 ${
              loading ? "opacity-50" :
              capacityStatus === "accepting"
                ? "bg-emerald-100 text-emerald-800"
                : capacityStatus === "paused"
                  ? "bg-orange-100 text-orange-800"
                  : capacityStatus === "at_capacity"
                    ? "bg-rose-100 text-rose-800"
                    : "bg-stone-100 text-stone-600"
            }`}
            style={{ backgroundImage: "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 8 8'%3E%3Cpath d='M0 2l4 4 4-4z' fill='%23666'/%3E%3C/svg%3E\")", backgroundRepeat: "no-repeat", backgroundPosition: "right 4px center" }}
          >
            <option value="accepting">accepting</option>
            <option value="paused">paused</option>
            <option value="at_capacity">at capacity</option>
          </select>
        )}
        <span className={`px-2 py-0.5 text-xs rounded-full font-medium shrink-0 ${
          post.postType === "service"
            ? "bg-blue-100 text-blue-800"
            : post.postType === "opportunity"
              ? "bg-purple-100 text-purple-800"
              : post.postType === "business"
                ? "bg-amber-100 text-amber-800"
                : "bg-stone-100 text-stone-600"
        }`}>
          {post.postType}
        </span>
        {post.relevanceScore != null && (
          <span className={`px-1.5 py-0.5 text-xs rounded font-bold shrink-0 ${
            post.relevanceScore >= 8
              ? "bg-green-100 text-green-800"
              : post.relevanceScore >= 5
                ? "bg-amber-100 text-amber-800"
                : "bg-red-100 text-red-800"
          }`}>
            {post.relevanceScore}
          </span>
        )}
      </div>
      <span className="text-sm text-stone-500 shrink-0 ml-3">
        {new Date(post.createdAt).toLocaleDateString()}
      </span>
    </div>
  );
}

function AddSocialProfileForm({
  orgId,
}: {
  orgId: string;
}) {
  const [platform, setPlatform] = useState("instagram");
  const [handle, setHandle] = useState("");
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [, createSocialSource] = useMutation(CreateSocialSourceMutation);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!handle.trim()) return;

    setLoading(true);
    setError(null);
    try {
      const result = await createSocialSource(
        {
          organizationId: orgId,
          platform: platform,
          identifier: handle.trim(),
        },
        { additionalTypenames: ["Source", "SourceConnection", "Organization"] },
      );
      if (result.error) throw result.error;
      setHandle("");
      setUrl("");
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
}: {
  noteableType: string;
  noteableId: string;
}) {
  const [content, setContent] = useState("");
  const [ctaText, setCtaText] = useState("");
  const [severity, setSeverity] = useState("info");
  const [isPublic, setIsPublic] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [, createNote] = useMutation(CreateNoteMutation);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim()) return;

    setLoading(true);
    setError(null);
    try {
      const result = await createNote(
        {
          content: content.trim(),
          severity,
          isPublic,
          ctaText: ctaText.trim() || null,
          noteableType,
          noteableId,
        },
        noteMutationContext,
      );
      if (result.error) throw result.error;
      setContent("");
      setCtaText("");
      setSeverity("info");
      setIsPublic(false);
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

type NoteData = {
  id: string;
  content: string;
  ctaText?: string | null;
  severity: string;
  sourceUrl?: string | null;
  sourceType?: string | null;
  isPublic: boolean;
  createdBy: string;
  expiredAt?: string | null;
  createdAt: string;
  linkedPosts?: { id: string; title: string }[] | null;
};

function NoteRow({
  note,
  noteableType,
  noteableId,
}: {
  note: NoteData;
  noteableType: string;
  noteableId: string;
}) {
  const isExpired = !!note.expiredAt;

  const [, updateNote] = useMutation(UpdateNoteMutation);
  const [, deleteNote] = useMutation(DeleteNoteMutation);
  const [, unlinkNote] = useMutation(UnlinkNoteMutation);

  const handleTogglePublic = async () => {
    try {
      const result = await updateNote(
        {
          id: note.id,
          content: note.content,
          severity: note.severity,
          isPublic: !note.isPublic,
          ctaText: note.ctaText,
        },
        noteMutationContext,
      );
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to toggle visibility:", err);
    }
  };

  const handleSeverityChange = async (newSeverity: string) => {
    try {
      const result = await updateNote(
        {
          id: note.id,
          content: note.content,
          severity: newSeverity,
          isPublic: note.isPublic,
          ctaText: note.ctaText,
        },
        noteMutationContext,
      );
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to update severity:", err);
    }
  };

  const handleDelete = async () => {
    try {
      const result = await deleteNote({ id: note.id }, noteMutationContext);
      if (result.error) throw result.error;
    } catch (err: any) {
      console.error("Failed to delete note:", err);
    }
  };

  const handleUnlink = async () => {
    try {
      const result = await unlinkNote(
        { noteId: note.id, postId: noteableId },
        noteMutationContext,
      );
      if (result.error) throw result.error;
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
              note.isPublic
                ? "bg-green-100 text-green-800 hover:bg-green-200"
                : "bg-stone-100 text-stone-500 hover:bg-stone-200"
            }`}
          >
            {note.isPublic ? "public" : "internal"}
          </button>
          {isExpired && (
            <span className="px-2 py-0.5 text-xs rounded-full font-medium bg-stone-200 text-stone-600">
              expired
            </span>
          )}
          {note.sourceType && (
            <span className="text-xs text-stone-400">
              via {note.sourceType}
            </span>
          )}
          <span className="text-xs text-stone-400">
            {note.createdBy} &middot; {new Date(note.createdAt).toLocaleDateString()}
          </span>
        </div>
        <p className="text-sm text-stone-700">{note.content}</p>
        {note.ctaText && (
          <p className="text-xs italic text-stone-500 mt-0.5">{note.ctaText}</p>
        )}
        {note.sourceUrl && (
          <a
            href={note.sourceUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs text-blue-600 hover:text-blue-800 mt-1 inline-block"
          >
            Source {"\u2197"}
          </a>
        )}
        {note.linkedPosts && note.linkedPosts.length > 0 && (
          <div className="flex flex-wrap items-center gap-1 mt-1.5">
            <span className="text-xs text-stone-400">Attached to:</span>
            {note.linkedPosts.map((post) => (
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
