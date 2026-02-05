"use client";

import Link from "next/link";
import { useState, useRef, useEffect } from "react";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useGraphQL, graphqlMutateClient, invalidateAllMatchingQuery } from "@/lib/graphql/client";
import { GET_WEBSITE_WITH_SNAPSHOTS, GET_WEBSITE_ASSESSMENT, GET_ALL_WEBSITES } from "@/lib/graphql/queries";
import { APPROVE_WEBSITE, REJECT_WEBSITE, CRAWL_WEBSITE, GENERATE_WEBSITE_ASSESSMENT, REGENERATE_POSTS } from "@/lib/graphql/mutations";

interface Tag {
  id: string;
  kind: string;
  value: string;
  displayName: string | null;
}

interface Listing {
  id: string;
  title: string;
  status: string;
  createdAt: string;
  sourceUrl: string | null;
  tags: Tag[];
}

interface Snapshot {
  url: string;
  siteUrl: string;
  title: string | null;
  content: string;
  fetchedAt: string;
  listingsCount: number;
}

interface WebsiteDetail {
  id: string;
  domain: string;
  status: string;
  submittedBy: string | null;
  submitterType: string;
  lastScrapedAt: string | null;
  snapshotsCount: number;
  listingsCount: number;
  createdAt: string;
  crawlStatus: string | null;
  crawlAttemptCount: number | null;
  maxCrawlRetries: number | null;
  lastCrawlStartedAt: string | null;
  lastCrawlCompletedAt: string | null;
  pagesCrawledCount: number | null;
  maxPagesPerCrawl: number | null;
  snapshots: Snapshot[];
  listings: Listing[];
}

interface Assessment {
  id: string;
  websiteId: string;
  assessmentMarkdown: string;
  recommendation: string;
  confidenceScore: number | null;
  organizationName: string | null;
  foundedYear: number | null;
  generatedAt: string;
  modelUsed: string;
  reviewedByHuman: boolean;
}

interface GetWebsiteResult {
  website: WebsiteDetail | null;
}

interface GetAssessmentResult {
  websiteAssessment: Assessment | null;
}

type TabType = "listings" | "snapshots" | "assessment";

export default function WebsiteDetailPage() {
  const params = useParams();
  const websiteId = params.id as string;
  const [activeTab, setActiveTab] = useState<TabType>("snapshots");
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
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

  const {
    data: websiteData,
    isLoading: websiteLoading,
    error: websiteError,
    mutate: refetchWebsite,
  } = useGraphQL<GetWebsiteResult>(GET_WEBSITE_WITH_SNAPSHOTS, { id: websiteId }, { revalidateOnFocus: false });

  const {
    data: assessmentData,
    mutate: refetchAssessment,
  } = useGraphQL<GetAssessmentResult>(GET_WEBSITE_ASSESSMENT, { websiteId }, { revalidateOnFocus: false });

  const website = websiteData?.website;
  const assessment = assessmentData?.websiteAssessment;

  const handleApprove = async () => {
    if (!confirm("Approve this website for crawling?")) return;

    setActionInProgress("approve");
    try {
      await graphqlMutateClient(APPROVE_WEBSITE, { websiteId });
      invalidateAllMatchingQuery(GET_ALL_WEBSITES);
      refetchWebsite();
    } catch (err) {
      console.error("Failed to approve:", err);
      alert("Failed to approve website");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    const reason = prompt("Reason for rejection:");
    if (reason === null) return;

    setActionInProgress("reject");
    try {
      await graphqlMutateClient(REJECT_WEBSITE, { websiteId, reason: reason || "Rejected" });
      invalidateAllMatchingQuery(GET_ALL_WEBSITES);
      refetchWebsite();
    } catch (err) {
      console.error("Failed to reject:", err);
      alert("Failed to reject website");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleCrawl = async () => {
    setActionInProgress("crawl");
    try {
      await graphqlMutateClient(CRAWL_WEBSITE, { websiteId });
      refetchWebsite();
    } catch (err) {
      console.error("Failed to start crawl:", err);
      alert("Failed to start crawl");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleGenerateAssessment = async () => {
    setActionInProgress("assessment");
    setMenuOpen(false);
    try {
      await graphqlMutateClient(GENERATE_WEBSITE_ASSESSMENT, { websiteId });
      refetchAssessment();
    } catch (err) {
      console.error("Failed to generate assessment:", err);
      alert("Failed to generate assessment");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleRegeneratePosts = async () => {
    setActionInProgress("regenerate");
    setMenuOpen(false);
    try {
      await graphqlMutateClient(REGENERATE_POSTS, { websiteId });
      alert("Post regeneration started");
      refetchWebsite();
    } catch (err) {
      console.error("Failed to regenerate posts:", err);
      alert("Failed to regenerate posts");
    } finally {
      setActionInProgress(null);
    }
  };

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

  const formatDate = (dateString: string | null) => {
    if (!dateString) return "Never";
    return new Date(dateString).toLocaleString();
  };

  if (websiteLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading website...</div>
      </div>
    );
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
              <h1 className="text-2xl font-bold text-stone-900 mb-2">{website.domain}</h1>
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
                    className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
                  >
                    {actionInProgress === "approve" ? "..." : "Approve"}
                  </button>
                  <button
                    onClick={handleReject}
                    disabled={actionInProgress !== null}
                    className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
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
                  {actionInProgress ? "..." : "⋯"}
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
                      onClick={handleRegeneratePosts}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      Regenerate Posts
                    </button>
                    <button
                      onClick={handleGenerateAssessment}
                      disabled={actionInProgress !== null}
                      className="w-full text-left px-4 py-2 text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50"
                    >
                      Generate AI Assessment
                    </button>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Stats Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Pages Crawled</span>
              <p className="text-lg font-semibold text-stone-900">
                {website.pagesCrawledCount || 0} / {website.maxPagesPerCrawl || 20}
              </p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Snapshots</span>
              <p className="text-lg font-semibold text-stone-900">{website.snapshotsCount || 0}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Posts</span>
              <p className="text-lg font-semibold text-stone-900">{website.listingsCount || 0}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Last Scraped</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(website.lastScrapedAt)}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(website.createdAt)}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Crawl Status</span>
              <p className="text-sm font-medium text-stone-900">{website.crawlStatus || "Idle"}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Submitted By</span>
              <p className="text-sm font-medium text-stone-900">
                {website.submittedBy || "Unknown"} ({website.submitterType})
              </p>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          <div className="border-b border-stone-200">
            <nav className="flex">
              {(["snapshots", "listings", "assessment"] as TabType[]).map((tab) => (
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
                  {tab === "snapshots" && ` (${website.snapshots.length})`}
                  {tab === "listings" && ` (${website.listings.length})`}
                </button>
              ))}
            </nav>
          </div>

          <div className="p-6">
            {/* Snapshots Tab */}
            {activeTab === "snapshots" && (
              <div className="space-y-4">
                {website.snapshots.length === 0 ? (
                  <div className="text-center py-8 text-stone-500">No snapshots yet</div>
                ) : (
                  website.snapshots.map((snapshot, index) => (
                    <div key={index} className="border border-stone-200 rounded-lg p-4">
                      <div className="flex justify-between items-start mb-2">
                        <div>
                          <h3 className="font-medium text-stone-900">{snapshot.title || "Untitled"}</h3>
                          <a
                            href={snapshot.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-sm text-blue-600 hover:underline"
                          >
                            {snapshot.url}
                          </a>
                        </div>
                        <span className="text-xs text-stone-500">
                          {snapshot.listingsCount} posts | {formatDate(snapshot.fetchedAt)}
                        </span>
                      </div>
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
                          {snapshot.content}
                        </ReactMarkdown>
                      </div>
                    </div>
                  ))
                )}
              </div>
            )}

            {/* Listings Tab */}
            {activeTab === "listings" && (
              <div className="space-y-4">
                {website.listings.length === 0 ? (
                  <div className="text-center py-8 text-stone-500">No posts yet</div>
                ) : (
                  website.listings.map((listing) => (
                    <Link
                      key={listing.id}
                      href={`/admin/posts/${listing.id}`}
                      className="block border border-stone-200 rounded-lg p-4 hover:bg-stone-50"
                    >
                      <div className="flex justify-between items-start">
                        <div>
                          <h3 className="font-medium text-stone-900">{listing.title}</h3>
                          <div className="flex gap-2 mt-2">
                            <span
                              className={`text-xs px-2 py-1 rounded ${
                                listing.status === "active"
                                  ? "bg-green-100 text-green-800"
                                  : listing.status === "pending_approval"
                                    ? "bg-amber-100 text-amber-800"
                                    : "bg-stone-100 text-stone-800"
                              }`}
                            >
                              {listing.status}
                            </span>
                            {listing.tags
                              .filter((t) => t.kind === "audience_role")
                              .map((tag) => (
                                <span
                                  key={tag.id}
                                  className="text-xs px-2 py-1 rounded bg-blue-100 text-blue-800"
                                >
                                  {tag.displayName || tag.value}
                                </span>
                              ))}
                          </div>
                        </div>
                        <span className="text-xs text-stone-500">{formatDate(listing.createdAt)}</span>
                      </div>
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
                      <div>
                        <h3 className="font-semibold text-stone-900">
                          {assessment.organizationName || "Assessment"}
                        </h3>
                        <p className="text-sm text-stone-500">
                          Generated {formatDate(assessment.generatedAt)} using {assessment.modelUsed}
                        </p>
                      </div>
                      <span
                        className={`px-3 py-1 text-sm rounded-full font-medium ${
                          assessment.recommendation === "approve"
                            ? "bg-green-100 text-green-800"
                            : assessment.recommendation === "reject"
                              ? "bg-red-100 text-red-800"
                              : "bg-amber-100 text-amber-800"
                        }`}
                      >
                        {assessment.recommendation}
                        {assessment.confidenceScore && ` (${Math.round(assessment.confidenceScore * 100)}%)`}
                      </span>
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
                        {assessment.assessmentMarkdown}
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
    </div>
  );
}
