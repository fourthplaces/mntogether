"use client";

import Link from "next/link";
import { useState, useRef, useEffect } from "react";
import { AdminLoader } from "@/components/admin/AdminLoader";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useRestateObject, useRestate, callObject, callService, invalidateService, invalidateObject } from "@/lib/restate/client";
import type {
  SourceObjectResult,
  OptionalAssessmentResult,
  ExtractionPageListResult,
  ExtractionPageCount,
  OrganizationResult,
  OrganizationListResult,
} from "@/lib/restate/types";

// Source object returns same shape as Extraction service
type SourcePageListResult = ExtractionPageListResult;
type SourcePageCountResult = ExtractionPageCount;

type TabType = "snapshots" | "assessment";

const SOURCE_TYPE_LABELS: Record<string, string> = {
  website: "Website",
  instagram: "Instagram",
  facebook: "Facebook",
  tiktok: "TikTok",
  x: "X (Twitter)",
};

export default function SourceDetailPage() {
  const params = useParams();
  const sourceId = params.id as string;
  const [activeTab, setActiveTab] = useState<TabType>("snapshots");
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [regenWorkflowId, setRegenWorkflowId] = useState<string | null>(null);
  const [regenStatus, setRegenStatus] = useState<string | null>(null);
  const [dedupWorkflowId, setDedupWorkflowId] = useState<string | null>(null);
  const [dedupStatus, setDedupStatus] = useState<string | null>(null);
  const [showOrgPicker, setShowOrgPicker] = useState(false);
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

  // --- Data hooks ---

  const {
    data: source,
    isLoading: sourceLoading,
    error: sourceError,
    mutate: refetchSource,
  } = useRestateObject<SourceObjectResult>("Source", sourceId, "get", {}, { revalidateOnFocus: false });

  const isWebsite = source?.source_type === "website";

  const {
    data: pagesData,
  } = useRestateObject<SourcePageListResult>(
    "Source", source ? sourceId : null, "list_pages", {},
    { revalidateOnFocus: false }
  );

  const {
    data: pageCount,
  } = useRestateObject<SourcePageCountResult>(
    "Source", source ? sourceId : null, "count_pages", {},
    { revalidateOnFocus: false }
  );

  const {
    data: assessmentData,
    mutate: refetchAssessment,
  } = useRestateObject<OptionalAssessmentResult>(
    "Source",
    isWebsite ? sourceId : null,
    "get_assessment",
    {},
    { revalidateOnFocus: false }
  );

  const { data: orgData } = useRestate<OrganizationResult>(
    source?.organization_id ? "Organizations" : null,
    "get",
    { id: source?.organization_id },
    { revalidateOnFocus: false }
  );

  const { data: orgsListData } = useRestate<OrganizationListResult>(
    showOrgPicker ? "Organizations" : null,
    "list",
    {},
    { revalidateOnFocus: false }
  );

  const assessment = assessmentData?.assessment ?? null;
  const pages = pagesData?.pages || [];

  // --- Actions ---

  const handleApprove = async () => {
    setActionInProgress("approve");
    try {
      await callObject("Source", sourceId, "approve", {});
      invalidateService("Sources");
      invalidateObject("Source", sourceId);
      refetchSource();
    } catch (err) {
      console.error("Failed to approve:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    setActionInProgress("reject");
    try {
      await callObject("Source", sourceId, "reject", { reason: "Rejected" });
      invalidateService("Sources");
      invalidateObject("Source", sourceId);
      refetchSource();
    } catch (err) {
      console.error("Failed to reject:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleCrawl = async () => {
    setActionInProgress("crawl");
    try {
      const workflowId = `crawl-${sourceId}-${Date.now()}`;
      if (isWebsite) {
        await callObject("CrawlWebsiteWorkflow", workflowId, "run", {
          website_id: sourceId,
        });
      } else {
        await callObject("CrawlSocialSourceWorkflow", workflowId, "run", {
          source_id: sourceId,
        });
      }
      refetchSource();
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
      await callObject("Source", sourceId, "generate_assessment", {});
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
      const result = await callObject<{ status: string }>("Source", sourceId, "regenerate_posts", {});
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
      const result = await callObject<{ status: string }>("Source", sourceId, "deduplicate_posts", {});
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
      await callObject("Source", sourceId, "extract_organization", {});
      invalidateObject("Source", sourceId);
      invalidateService("Sources");
      invalidateService("Organizations");
      refetchSource();
    } catch (err) {
      console.error("Failed to extract organization:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  const handleAssignOrganization = async (orgId: string) => {
    setActionInProgress("assign_org");
    try {
      await callObject("Source", sourceId, "assign_organization", { organization_id: orgId });
      invalidateObject("Source", sourceId);
      invalidateService("Sources");
      invalidateService("Organizations");
      refetchSource();
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
      await callObject("Source", sourceId, "unassign_organization", {});
      invalidateObject("Source", sourceId);
      invalidateService("Sources");
      invalidateService("Organizations");
      refetchSource();
    } catch (err) {
      console.error("Failed to unassign organization:", err);
    } finally {
      setActionInProgress(null);
    }
  };

  // Poll regenerate posts workflow status
  const regenWorkflowName = isWebsite ? "RegeneratePostsWorkflow" : "RegenerateSocialPostsWorkflow";
  useEffect(() => {
    if (!regenWorkflowId) return;
    const interval = setInterval(async () => {
      try {
        const status = await callObject<string>(regenWorkflowName, regenWorkflowId, "get_status", {});
        setRegenStatus(status);
        if (status.startsWith("Completed:") || status.startsWith("Completed ") || status.startsWith("Failed:")) {
          clearInterval(interval);
          setRegenWorkflowId(null);
          setActionInProgress(null);
        }
      } catch { /* keep polling */ }
    }, 3000);
    return () => clearInterval(interval);
  }, [regenWorkflowId, regenWorkflowName]);

  // Poll deduplicate posts workflow status
  useEffect(() => {
    if (!dedupWorkflowId) return;
    const interval = setInterval(async () => {
      try {
        const status = await callObject<string>("DeduplicatePostsWorkflow", dedupWorkflowId, "get_status", {});
        setDedupStatus(status);
        if (status.startsWith("Completed:") || status.startsWith("Completed ") || status.startsWith("Failed:")) {
          clearInterval(interval);
          setDedupWorkflowId(null);
          setActionInProgress(null);
        }
      } catch { /* keep polling */ }
    }, 3000);
    return () => clearInterval(interval);
  }, [dedupWorkflowId]);

  // --- Helpers ---

  const getStatusColor = (status: string) => {
    switch (status?.toLowerCase()) {
      case "approved": return "bg-green-100 text-green-800";
      case "pending_review": case "pending": return "bg-yellow-100 text-yellow-800";
      case "rejected": return "bg-red-100 text-red-800";
      case "suspended": return "bg-gray-100 text-gray-800";
      default: return "bg-gray-100 text-gray-800";
    }
  };

  const formatDate = (dateString: string | null | undefined) => {
    if (!dateString) return "Never";
    return new Date(dateString).toLocaleString();
  };

  // Determine available tabs — all sources have snapshots, assessment is website-only
  const tabs: TabType[] = isWebsite
    ? ["snapshots", "assessment"]
    : ["snapshots"];

  // --- Loading / Error states ---

  if (sourceLoading) {
    return <AdminLoader label="Loading source..." />;
  }

  if (sourceError) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-6xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Source</h1>
          <p className="text-stone-600 mb-4">{sourceError.message}</p>
          <Link href="/admin/sources" className="text-blue-600 hover:text-blue-800">
            Back to Sources
          </Link>
        </div>
      </div>
    );
  }

  if (!source) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-6xl mx-auto text-center py-12">
          <h1 className="text-2xl font-bold text-stone-900 mb-4">Source Not Found</h1>
          <Link href="/admin/sources" className="text-blue-600 hover:text-blue-800">
            Back to Sources
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-6xl mx-auto">
        <Link
          href="/admin/sources"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Sources
        </Link>

        {/* Source Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start mb-4">
            <div>
              <div className="flex items-center gap-3 mb-2">
                <h1 className="text-2xl font-bold text-stone-900">{source.identifier}</h1>
                <span className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                  source.source_type === "website" ? "bg-blue-100 text-blue-800" :
                  source.source_type === "instagram" ? "bg-purple-100 text-purple-800" :
                  source.source_type === "facebook" ? "bg-indigo-100 text-indigo-800" :
                  source.source_type === "x" ? "bg-stone-800 text-white" :
                  "bg-stone-100 text-stone-800"
                }`}>
                  {SOURCE_TYPE_LABELS[source.source_type] || source.source_type}
                </span>
                {source.url && (
                  <a
                    href={source.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm text-blue-600 hover:text-blue-800"
                  >
                    Visit {"\u2197"}
                  </a>
                )}
              </div>
              <span className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusColor(source.status)}`}>
                {source.status}
              </span>
            </div>
            <div className="flex gap-2">
              {source.status === "pending_review" && (
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
                    {isWebsite && (
                      <button
                        onClick={handleGenerateAssessment}
                        disabled={actionInProgress !== null}
                        className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                      >
                        Generate AI Assessment
                      </button>
                    )}
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
                    {isWebsite && !source.organization_id && (
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
                    {source.organization_id && (
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
              ) : isWebsite ? (
                <button
                  onClick={handleExtractOrganization}
                  disabled={actionInProgress !== null}
                  className="block text-sm text-amber-600 hover:text-amber-800 font-medium disabled:opacity-50"
                >
                  Extract with AI
                </button>
              ) : (
                <p className="text-sm text-stone-400">{"\u2014"}</p>
              )}
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Pages Crawled</span>
              <p className="text-lg font-semibold text-stone-900">{pageCount?.count ?? 0}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Last Scraped</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(source.last_scraped_at)}</p>
            </div>
          </div>

          {regenStatus && actionInProgress === "regenerate" && (
            <div className="mt-4 pt-4 border-t border-stone-200">
              <div className="flex items-center gap-3">
                <div className="animate-spin h-4 w-4 border-2 border-amber-600 border-t-transparent rounded-full" />
                <span className="text-sm font-medium text-amber-700">{regenStatus}</span>
              </div>
            </div>
          )}

          {dedupStatus && actionInProgress === "deduplicate" && (
            <div className="mt-4 pt-4 border-t border-stone-200">
              <div className="flex items-center gap-3">
                <div className="animate-spin h-4 w-4 border-2 border-blue-600 border-t-transparent rounded-full" />
                <span className="text-sm font-medium text-blue-700">{dedupStatus}</span>
              </div>
            </div>
          )}
        </div>

        {/* Tabs (website only) */}
        {tabs.length > 0 && (
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          <div className="border-b border-stone-200">
            <nav className="flex">
              {tabs.map((tab) => (
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
                  {tab === "snapshots" && ` (${pageCount?.count ?? pages.length})`}
                </button>
              ))}
            </nav>
          </div>

          <div className="p-6">
            {/* Snapshots Tab */}
            {activeTab === "snapshots" && (
              <div className="space-y-4">
                {pages.length === 0 ? (
                  <div className="text-center py-8 text-stone-500">No crawled pages yet</div>
                ) : (
                  pages.map((page, index) => (
                    <Link
                      key={index}
                      href={`/admin/sources/${sourceId}/snapshots?url=${encodeURIComponent(page.url)}`}
                      className="block border border-stone-200 rounded-lg p-4 hover:border-stone-300 hover:shadow-sm transition-all"
                    >
                      <div className="mb-2">
                        <span className="text-sm text-blue-600">{page.url}</span>
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

            {/* Assessment Tab (website only) */}
            {activeTab === "assessment" && isWebsite && (
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
        )}
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
                          source?.organization_id === org.id
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
