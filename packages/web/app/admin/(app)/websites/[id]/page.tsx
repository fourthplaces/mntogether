"use client";

import Link from "next/link";
import { useState, useRef, useEffect } from "react";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useRestateObject, useRestate, callObject, callService, invalidateService, invalidateObject } from "@/lib/restate/client";
import type {
  WebsiteResult,
  OptionalAssessmentResult,
  PostList,
  ExtractionPageListResult,
  ExtractionPageCount,
  OrganizationResult,
  OrganizationListResult,
} from "@/lib/restate/types";

type TabType = "posts" | "snapshots" | "assessment";

export default function WebsiteDetailPage() {
  const params = useParams();
  const websiteId = params.id as string;
  const [activeTab, setActiveTab] = useState<TabType>("posts");
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [regenWorkflowId, setRegenWorkflowId] = useState<string | null>(null);
  const [regenStatus, setRegenStatus] = useState<string | null>(null);
  const [dedupWorkflowId, setDedupWorkflowId] = useState<string | null>(null);
  const [dedupStatus, setDedupStatus] = useState<string | null>(null);
  const [approvingPostId, setApprovingPostId] = useState<string | null>(null);
  const [rejectingPostId, setRejectingPostId] = useState<string | null>(null);
  const [showOrgPicker, setShowOrgPicker] = useState(false);
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

  // --- Data hooks ---

  const {
    data: website,
    isLoading: websiteLoading,
    error: websiteError,
    mutate: refetchWebsite,
  } = useRestateObject<WebsiteResult>("Website", websiteId, "get", {}, { revalidateOnFocus: false });

  const {
    data: postsData,
    mutate: refetchPosts,
  } = useRestate<PostList>(
    "Posts", "list",
    { website_id: websiteId, first: 100 },
    { revalidateOnFocus: false }
  );

  const {
    data: pagesData,
  } = useRestate<ExtractionPageListResult>(
    website?.domain ? "Extraction" : null,
    "list_pages",
    { domain: website?.domain, limit: 50 },
    { revalidateOnFocus: false }
  );

  const {
    data: pageCount,
  } = useRestate<ExtractionPageCount>(
    website?.domain ? "Extraction" : null,
    "count_pages",
    { domain: website?.domain },
    { revalidateOnFocus: false }
  );

  const {
    data: assessmentData,
    mutate: refetchAssessment,
  } = useRestateObject<OptionalAssessmentResult>("Website", websiteId, "get_assessment", {}, { revalidateOnFocus: false });

  const { data: orgData } = useRestate<OrganizationResult>(
    website?.organization_id ? "Organizations" : null,
    "get",
    { id: website?.organization_id },
    { revalidateOnFocus: false }
  );

  const { data: orgsListData } = useRestate<OrganizationListResult>(
    showOrgPicker ? "Organizations" : null,
    "list",
    {},
    { revalidateOnFocus: false }
  );

  const assessment = assessmentData?.assessment ?? null;

  const posts = postsData?.posts || [];
  const pages = pagesData?.pages || [];

  // --- Actions ---

  const handleApprove = async () => {
    setActionInProgress("approve");
    try {
      await callObject("Website", websiteId, "approve", {});
      invalidateService("Websites");
      invalidateObject("Website", websiteId);
      refetchWebsite();
    } catch (err) {
      console.error("Failed to approve:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    setActionInProgress("reject");
    try {
      await callObject("Website", websiteId, "reject", { reason: "Rejected" });
      invalidateService("Websites");
      invalidateObject("Website", websiteId);
      refetchWebsite();
    } catch (err) {
      console.error("Failed to reject:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleApprovePost = async (postId: string) => {
    setApprovingPostId(postId);
    try {
      await callObject("Post", postId, "approve", {});
      invalidateService("Posts");
      invalidateObject("Post", postId);
      refetchPosts();
    } catch (err) {
      console.error("Failed to approve post:", err);
    } finally {
      setApprovingPostId(null);
    }
  };

  const handleRejectPost = async (postId: string) => {
    setRejectingPostId(postId);
    try {
      await callObject("Post", postId, "reject", { reason: "Rejected by admin" });
      invalidateService("Posts");
      invalidateObject("Post", postId);
      refetchPosts();
    } catch (err) {
      console.error("Failed to reject post:", err);
    } finally {
      setRejectingPostId(null);
    }
  };

  const handleCrawl = async () => {
    setActionInProgress("crawl");
    try {
      const workflowId = `crawl-${websiteId}-${Date.now()}`;
      await callObject("CrawlWebsiteWorkflow", workflowId, "run", {
        website_id: websiteId,
        visitor_id: "00000000-0000-0000-0000-000000000000",
      });
      refetchWebsite();
    } catch (err) {
      console.error("Failed to start crawl:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleGenerateAssessment = async () => {
    setActionInProgress("assessment");
    setMenuOpen(false);
    try {
      await callObject("Website", websiteId, "generate_assessment", {});
      refetchAssessment();
    } catch (err) {
      console.error("Failed to generate assessment:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleRegeneratePosts = async () => {
    setActionInProgress("regenerate");
    setMenuOpen(false);
    try {
      const result = await callObject<{ status: string }>("Website", websiteId, "regenerate_posts", {});
      // status is "started:{workflow_id}"
      const workflowId = result.status.replace("started:", "");
      setRegenWorkflowId(workflowId);
      setRegenStatus("Starting...");
    } catch (err) {
      console.error("Failed to start regeneration:", err);
      setActionInProgress(null);
    }
  };

  const handleDeduplicatePosts = async () => {
    setActionInProgress("deduplicate");
    setMenuOpen(false);
    try {
      const result = await callObject<{ status: string }>("Website", websiteId, "deduplicate_posts", {});
      const workflowId = result.status.replace("started:", "");
      setDedupWorkflowId(workflowId);
      setDedupStatus("Starting...");
    } catch (err) {
      console.error("Failed to start deduplication:", err);
      setActionInProgress(null);
    }
  };

  const handleExtractOrganization = async () => {
    setActionInProgress("extract_org");
    setMenuOpen(false);
    try {
      const result = await callObject<{ organization_id: string | null; status: string }>(
        "Website", websiteId, "extract_organization", {}
      );
      invalidateObject("Website", websiteId);
      invalidateService("Websites");
      invalidateService("Organizations");
      refetchWebsite();
    } catch (err) {
      console.error("Failed to extract organization:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleAssignOrganization = async (orgId: string) => {
    setActionInProgress("assign_org");
    try {
      await callObject("Website", websiteId, "assign_organization", { organization_id: orgId });
      invalidateObject("Website", websiteId);
      invalidateService("Websites");
      invalidateService("Organizations");
      refetchWebsite();
      setShowOrgPicker(false);
    } catch (err) {
      console.error("Failed to assign organization:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleUnassignOrganization = async () => {
    setActionInProgress("unassign_org");
    try {
      await callObject("Website", websiteId, "unassign_organization", {});
      invalidateObject("Website", websiteId);
      invalidateService("Websites");
      invalidateService("Organizations");
      refetchWebsite();
    } catch (err) {
      console.error("Failed to unassign organization:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  // Poll regenerate posts workflow status
  useEffect(() => {
    if (!regenWorkflowId) return;

    const interval = setInterval(async () => {
      try {
        const status = await callObject<string>(
          "RegeneratePostsWorkflow", regenWorkflowId, "get_status", {}
        );
        setRegenStatus(status);

        if (status.startsWith("Completed:") || status.startsWith("Completed ") || status.startsWith("Failed:")) {
          clearInterval(interval);
          setRegenWorkflowId(null);
          setActionInProgress(null);
          refetchPosts();
        }
      } catch {
        // Workflow may not be ready yet, keep polling
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [regenWorkflowId, refetchPosts]);

  // Poll deduplicate posts workflow status
  useEffect(() => {
    if (!dedupWorkflowId) return;

    const interval = setInterval(async () => {
      try {
        const status = await callObject<string>(
          "DeduplicatePostsWorkflow", dedupWorkflowId, "get_status", {}
        );
        setDedupStatus(status);

        if (status.startsWith("Completed:") || status.startsWith("Completed ") || status.startsWith("Failed:")) {
          clearInterval(interval);
          setDedupWorkflowId(null);
          setActionInProgress(null);
          refetchPosts();
        }
      } catch {
        // Workflow may not be ready yet, keep polling
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [dedupWorkflowId, refetchPosts]);

  // --- Helpers ---

  const getStatusColor = (status: string) => {
    switch (status?.toLowerCase()) {
      case "approved":
        return "bg-green-100 text-green-800";
      case "pending_review":
      case "pending":
        return "bg-yellow-100 text-yellow-800";
      case "rejected":
        return "bg-red-100 text-red-800";
      case "suspended":
        return "bg-gray-100 text-gray-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const formatStatus = (status: string) => {
    const map: Record<string, string> = {
      Active: "Active",
      active: "Active",
      PendingApproval: "Pending Approval",
      pending_approval: "Pending Approval",
      Rejected: "Rejected",
      rejected: "Rejected",
      Expired: "Expired",
      expired: "Expired",
      Filled: "Filled",
      filled: "Filled",
      Archived: "Archived",
      archived: "Archived",
    };
    return map[status] || status.replace(/_/g, " ").replace(/\b\w/g, c => c.toUpperCase());
  };

  const formatDate = (dateString: string | null | undefined) => {
    if (!dateString) return "Never";
    return new Date(dateString).toLocaleString();
  };

  // --- Loading / Error states ---

  if (websiteLoading) {
    return <AdminLoader label="Loading website..." />;
  }

  if (websiteError) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-6xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Website</h1>
            <p className="text-stone-600 mb-4">{websiteError.message}</p>
            <Link href="/admin/websites" className="text-blue-600 hover:text-blue-800">
              Back to Websites
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!website) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-6xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Website Not Found</h1>
            <Link href="/admin/websites" className="text-blue-600 hover:text-blue-800">
              Back to Websites
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-6xl mx-auto">
        {/* Back Button */}
        <Link
          href="/admin/websites"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Websites
        </Link>

        {/* Website Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start mb-4">
            <div>
              <div className="flex items-center gap-3 mb-2">
                <h1 className="text-2xl font-bold text-stone-900">{website.domain}</h1>
                <a
                  href={`https://${website.domain}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-blue-600 hover:text-blue-800"
                >
                  Visit site {"\u2197"}
                </a>
              </div>
              <span className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusColor(website.status)}`}>
                {website.status}
              </span>
            </div>
            <div className="flex gap-2">
              {website.status === "pending_review" && (
                <>
                  <button
                    onClick={handleApprove}
                    disabled={actionInProgress !== null}
                    className="px-4 py-2 bg-emerald-400 text-white rounded hover:bg-emerald-500 disabled:opacity-50"
                  >
                    {actionInProgress === "approve" ? "..." : "Approve"}
                  </button>
                  <button
                    onClick={handleReject}
                    disabled={actionInProgress !== null}
                    className="px-4 py-2 bg-rose-400 text-white rounded hover:bg-rose-500 disabled:opacity-50"
                  >
                    {actionInProgress === "reject" ? "..." : "Reject"}
                  </button>
                </>
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
                      onClick={() => { setMenuOpen(false); handleCrawl(); }}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      Start Crawl
                    </button>
                    <button
                      onClick={handleGenerateAssessment}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      Generate AI Assessment
                    </button>
                    <button
                      onClick={handleRegeneratePosts}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      Regenerate Posts
                    </button>
                    <button
                      onClick={handleDeduplicatePosts}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      Deduplicate Posts
                    </button>
                    <div className="border-t border-stone-100 my-1" />
                    {!website.organization_id && (
                      <button
                        onClick={handleExtractOrganization}
                        disabled={actionInProgress !== null}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        {actionInProgress === "extract_org" ? "Extracting..." : "Extract Organization (AI)"}
                      </button>
                    )}
                    <button
                      onClick={() => { setMenuOpen(false); setShowOrgPicker(true); }}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      Assign Organization
                    </button>
                    {website.organization_id && (
                      <button
                        onClick={() => { setMenuOpen(false); handleUnassignOrganization(); }}
                        disabled={actionInProgress !== null}
                        className="w-full text-left px-4 py-2 text-sm text-red-600 hover:bg-red-50 disabled:opacity-50"
                      >
                        Remove Organization
                      </button>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Stats Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Organization</span>
              {orgData ? (
                <Link
                  href={`/admin/organizations/${orgData.id}`}
                  className="block text-sm font-medium text-amber-700 hover:text-amber-900"
                >
                  {orgData.name}
                </Link>
              ) : actionInProgress === "extract_org" ? (
                <div className="flex items-center gap-2 mt-0.5">
                  <div className="animate-spin h-3 w-3 border-2 border-amber-600 border-t-transparent rounded-full" />
                  <span className="text-sm text-amber-600">Extracting...</span>
                </div>
              ) : (
                <button
                  onClick={handleExtractOrganization}
                  disabled={actionInProgress !== null}
                  className="block text-sm text-amber-600 hover:text-amber-800 font-medium disabled:opacity-50"
                >
                  Extract with AI
                </button>
              )}
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Posts</span>
              <p className="text-lg font-semibold text-stone-900">{postsData?.total_count ?? website.post_count ?? 0}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Pages Crawled</span>
              <p className="text-lg font-semibold text-stone-900">{pageCount?.count ?? 0}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Last Crawled</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(website.last_crawled_at)}</p>
            </div>
          </div>

          {/* Regeneration Progress */}
          {regenStatus && actionInProgress === "regenerate" && (
            <div className="mt-4 pt-4 border-t border-stone-200">
              <div className="flex items-center gap-3">
                <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
                <span className="text-sm font-medium text-amber-700">{regenStatus}</span>
              </div>
            </div>
          )}

          {/* Deduplication Progress */}
          {dedupStatus && actionInProgress === "deduplicate" && (
            <div className="mt-4 pt-4 border-t border-stone-200">
              <div className="flex items-center gap-3">
                <div className="animate-spin h-4 w-4 border-2 border-blue-600 border-t-transparent rounded-full" />
                <span className="text-sm font-medium text-blue-700">{dedupStatus}</span>
              </div>
            </div>
          )}
        </div>

        {/* Tabs */}
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          <div className="border-b border-stone-200">
            <nav className="flex">
              {(["posts", "snapshots", "assessment"] as TabType[]).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={`px-6 py-3 text-sm font-medium ${
                    activeTab === tab
                      ? "border-b-2 border-amber-600 text-amber-600"
                      : "text-stone-500 hover:text-stone-700"
                  }`}
                >
                  {tab.charAt(0).toUpperCase() + tab.slice(1)}
                  {tab === "posts" && ` (${postsData?.total_count ?? posts.length})`}
                  {tab === "snapshots" && ` (${pageCount?.count ?? pages.length})`}
                </button>
              ))}
            </nav>
          </div>

          <div className="p-6">
            {/* Posts Tab */}
            {activeTab === "posts" && (
              <div className="space-y-4">
                {posts.length === 0 ? (
                  <div className="text-center py-8 text-stone-500">No posts yet</div>
                ) : (
                  posts.map((post) => (
                    <div
                      key={post.id}
                      className="border border-stone-200 rounded-lg p-4 hover:bg-stone-50"
                    >
                      <div className="flex justify-between items-start">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <Link href={`/admin/posts/${post.id}`} className="font-medium text-stone-900 hover:underline">
                              {post.title}
                            </Link>
                            <span
                              className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                                post.status === "active" || post.status === "Active"
                                  ? "bg-green-100 text-green-800"
                                  : post.status === "pending_approval" || post.status === "PendingApproval"
                                    ? "bg-amber-100 text-amber-800"
                                    : "bg-stone-100 text-stone-600"
                              }`}
                            >
                              {formatStatus(post.status)}
                            </span>
                          </div>
                          {post.summary && (
                            <p className="text-sm text-stone-500 mt-1 line-clamp-2">{post.summary}</p>
                          )}
                          {post.tags && post.tags.length > 0 && (
                            <div className="flex flex-wrap gap-1.5 mt-2">
                              {post.tags.map((tag) => (
                                <span
                                  key={`${tag.kind}:${tag.value}`}
                                  className="text-xs px-2 py-1 rounded bg-stone-100 text-stone-600"
                                >
                                  {tag.kind}: {tag.display_name || tag.value}
                                </span>
                              ))}
                            </div>
                          )}
                        </div>
                        {(post.status === "pending_approval" || post.status === "PendingApproval") && (
                          <div className="flex items-center gap-2 ml-4 shrink-0">
                            <button
                              onClick={() => handleApprovePost(post.id)}
                              disabled={approvingPostId === post.id || rejectingPostId === post.id}
                              className="px-3 py-1.5 text-xs font-medium rounded-md bg-green-600 text-white hover:bg-green-700 disabled:opacity-50"
                            >
                              {approvingPostId === post.id ? "Approving..." : "Approve"}
                            </button>
                            <button
                              onClick={() => handleRejectPost(post.id)}
                              disabled={approvingPostId === post.id || rejectingPostId === post.id}
                              className="px-3 py-1.5 text-xs font-medium rounded-md bg-red-600 text-white hover:bg-red-700 disabled:opacity-50"
                            >
                              {rejectingPostId === post.id ? "Rejecting..." : "Reject"}
                            </button>
                          </div>
                        )}
                      </div>
                    </div>
                  ))
                )}
              </div>
            )}

            {/* Snapshots Tab */}
            {activeTab === "snapshots" && (
              <div className="space-y-4">
                {pages.length === 0 ? (
                  <div className="text-center py-8 text-stone-500">No crawled pages yet</div>
                ) : (
                  pages.map((page, index) => (
                    <Link
                      key={index}
                      href={`/admin/websites/${websiteId}/snapshots?url=${encodeURIComponent(page.url)}`}
                      className="block border border-stone-200 rounded-lg p-4 hover:border-stone-300 hover:shadow-sm transition-all"
                    >
                      <div className="mb-2">
                        <span className="text-sm text-blue-600">
                          {page.url}
                        </span>
                      </div>
                      {page.content && (
                        <div className="text-sm text-stone-600 line-clamp-3">
                          <ReactMarkdown
                            components={{
                              p: ({ children }) => <span>{children}</span>,
                              ul: ({ children }) => <span>{children}</span>,
                              ol: ({ children }) => <span>{children}</span>,
                              li: ({ children }) => <span>{children} </span>,
                              strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
                              a: ({ children }) => <span>{children}</span>,
                            }}
                          >
                            {page.content.slice(0, 500)}
                          </ReactMarkdown>
                        </div>
                      )}
                    </Link>
                  ))
                )}
              </div>
            )}

            {/* Assessment Tab */}
            {activeTab === "assessment" && (
              <div>
                {assessment ? (
                  <div className="space-y-4">
                    <div className="flex justify-between items-start">
                      <h3 className="font-semibold text-stone-900">Assessment</h3>
                      {assessment.confidence_score != null && (
                        <span className="px-3 py-1 text-sm rounded-full font-medium bg-blue-100 text-blue-800">
                          Confidence: {Math.round(assessment.confidence_score * 100)}%
                        </span>
                      )}
                    </div>
                    <div className="prose prose-stone max-w-none">
                      <ReactMarkdown
                        components={{
                          p: ({ children }) => <p className="mb-4 text-stone-700">{children}</p>,
                          ul: ({ children }) => <ul className="list-disc pl-6 mb-4 space-y-1">{children}</ul>,
                          ol: ({ children }) => <ol className="list-decimal pl-6 mb-4 space-y-1">{children}</ol>,
                          li: ({ children }) => <li className="text-stone-700">{children}</li>,
                          strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
                          h1: ({ children }) => <h1 className="text-xl font-bold text-stone-900 mt-6 mb-3">{children}</h1>,
                          h2: ({ children }) => <h2 className="text-lg font-bold text-stone-900 mt-5 mb-2">{children}</h2>,
                          h3: ({ children }) => <h3 className="text-base font-semibold text-stone-900 mt-4 mb-2">{children}</h3>,
                          a: ({ href, children }) => (
                            <a href={href} className="text-blue-600 hover:text-blue-800 underline" target="_blank" rel="noopener noreferrer">
                              {children}
                            </a>
                          ),
                        }}
                      >
                        {assessment.assessment_markdown}
                      </ReactMarkdown>
                    </div>
                  </div>
                ) : (
                  <div className="text-center py-8">
                    <p className="text-stone-500 mb-4">No assessment generated yet</p>
                    <button
                      onClick={handleGenerateAssessment}
                      disabled={actionInProgress !== null}
                      className="px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700 disabled:opacity-50"
                    >
                      {actionInProgress === "assessment" ? "Generating..." : "Generate Assessment"}
                    </button>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Assign Organization Modal */}
      {showOrgPicker && (
        <>
          <div
            className="fixed inset-0 bg-black/40 z-40"
            onClick={() => setShowOrgPicker(false)}
          />
          <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            <div className="bg-white rounded-xl shadow-xl w-full max-w-md max-h-[70vh] flex flex-col">
              <div className="flex items-center justify-between px-5 py-4 border-b border-stone-200">
                <h2 className="text-lg font-semibold text-stone-900">Assign Organization</h2>
                <button
                  onClick={() => setShowOrgPicker(false)}
                  className="text-stone-400 hover:text-stone-600 text-xl leading-none"
                >
                  {"\u00D7"}
                </button>
              </div>
              <div className="flex-1 overflow-y-auto p-5">
                {(orgsListData?.organizations || []).length === 0 ? (
                  <div className="text-center py-8">
                    <p className="text-stone-500 mb-3">No organizations yet.</p>
                    <Link
                      href="/admin/organizations"
                      className="text-amber-600 hover:text-amber-800 text-sm font-medium"
                      onClick={() => setShowOrgPicker(false)}
                    >
                      Create one first {"\u2192"}
                    </Link>
                  </div>
                ) : (
                  <div className="space-y-1">
                    {(orgsListData?.organizations || []).map((org) => (
                      <button
                        key={org.id}
                        onClick={() => handleAssignOrganization(org.id)}
                        disabled={actionInProgress !== null}
                        className={`w-full text-left px-4 py-3 rounded-lg transition-colors disabled:opacity-50 ${
                          website?.organization_id === org.id
                            ? "bg-amber-100 text-amber-900"
                            : "hover:bg-stone-50 text-stone-800"
                        }`}
                      >
                        <div className="font-medium">{org.name}</div>
                        {org.description && (
                          <div className="text-sm text-stone-500 mt-0.5 line-clamp-1">
                            {org.description}
                          </div>
                        )}
                        <div className="text-xs text-stone-400 mt-1">
                          {org.website_count} websites · {org.social_profile_count} social profiles
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
