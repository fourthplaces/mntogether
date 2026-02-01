import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, gql } from '@apollo/client';
import ReactMarkdown from 'react-markdown';
import { GET_WEBSITE_ASSESSMENT } from '../../graphql/queries';
import { GENERATE_WEBSITE_ASSESSMENT, CRAWL_WEBSITE } from '../../graphql/mutations';

const GET_WEBSITE_WITH_SNAPSHOTS = gql`
  query GetWebsiteWithSnapshots($id: Uuid!) {
    website(id: $id) {
      id
      domain
      status
      submittedBy
      submitterType
      lastScrapedAt
      snapshotsCount
      listingsCount
      createdAt
      crawlStatus
      crawlAttemptCount
      maxCrawlRetries
      lastCrawlStartedAt
      lastCrawlCompletedAt
      pagesCrawledCount
      maxPagesPerCrawl
      snapshots {
        id
        pageUrl
        pageSnapshotId
        scrapeStatus
        scrapeError
        lastScrapedAt
        submittedAt
        summary
      }
      listings {
        id
        title
        status
        createdAt
        sourceUrl
      }
    }
  }
`;

const APPROVE_WEBSITE = gql`
  mutation ApproveWebsite($websiteId: String!) {
    approveWebsite(websiteId: $websiteId) {
      id
      status
    }
  }
`;

const REJECT_WEBSITE = gql`
  mutation RejectWebsite($websiteId: String!, $reason: String!) {
    rejectWebsite(websiteId: $websiteId, reason: $reason) {
      id
      status
    }
  }
`;

const UPDATE_CRAWL_SETTINGS = gql`
  mutation UpdateWebsiteCrawlSettings($websiteId: String!, $maxPagesPerCrawl: Int!) {
    updateWebsiteCrawlSettings(websiteId: $websiteId, maxPagesPerCrawl: $maxPagesPerCrawl) {
      id
      maxPagesPerCrawl
    }
  }
`;

interface WebsiteSnapshot {
  id: string;
  pageUrl: string;
  pageSnapshotId: string | null;
  scrapeStatus: string;
  scrapeError: string | null;
  lastScrapedAt: string | null;
  submittedAt: string;
  summary: string | null;
}

interface Listing {
  id: string;
  title: string;
  status: string;
  createdAt: string;
  sourceUrl: string | null;
}

interface Website {
  id: string;
  domain: string;
  status: string;
  submittedBy: string | null;
  submitterType: string;
  lastScrapedAt: string | null;
  snapshotsCount: number;
  listingsCount: number;
  createdAt: string;
  // Crawl tracking
  crawlStatus: string | null;
  crawlAttemptCount: number | null;
  maxCrawlRetries: number | null;
  lastCrawlStartedAt: string | null;
  lastCrawlCompletedAt: string | null;
  pagesCrawledCount: number | null;
  maxPagesPerCrawl: number | null;
  snapshots: WebsiteSnapshot[];
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

type TabType = 'listings' | 'snapshots' | 'assessment';

export function WebsiteDetail() {
  const { websiteId } = useParams<{ websiteId: string }>();
  const [error, setError] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isCrawling, setIsCrawling] = useState(false);
  const [isEditingMaxPages, setIsEditingMaxPages] = useState(false);
  const [maxPagesInput, setMaxPagesInput] = useState<number>(20);
  const [activeTab, setActiveTab] = useState<TabType>('listings');
  const [showMoreMenu, setShowMoreMenu] = useState(false);

  const { data: websiteData, loading: websiteLoading, refetch: refetchWebsite } = useQuery<{
    website: Website | null;
  }>(GET_WEBSITE_WITH_SNAPSHOTS, {
    variables: { id: websiteId },
    skip: !websiteId,
  });

  const website = websiteData?.website;

  const { data: assessmentData, loading: assessmentLoading, refetch: refetchAssessment } = useQuery<{
    websiteAssessment: Assessment | null;
  }>(GET_WEBSITE_ASSESSMENT, {
    variables: { websiteId },
    skip: !websiteId,
  });

  const [generateAssessment] = useMutation(GENERATE_WEBSITE_ASSESSMENT, {
    onCompleted: () => {
      setIsGenerating(false);
      refetchAssessment();
    },
    onError: (err) => {
      setError(err.message);
      setIsGenerating(false);
    },
  });

  const [approveWebsite] = useMutation(APPROVE_WEBSITE, {
    onCompleted: () => refetchWebsite(),
    onError: (err) => setError(err.message),
  });

  const [rejectWebsite] = useMutation(REJECT_WEBSITE, {
    onCompleted: () => refetchWebsite(),
    onError: (err) => setError(err.message),
  });

  const [crawlWebsite] = useMutation(CRAWL_WEBSITE, {
    onCompleted: () => {
      setIsCrawling(false);
      refetchWebsite();
    },
    onError: (err) => {
      setError(err.message);
      setIsCrawling(false);
    },
  });

  const [updateCrawlSettings] = useMutation(UPDATE_CRAWL_SETTINGS, {
    onCompleted: () => {
      setIsEditingMaxPages(false);
      refetchWebsite();
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const handleGenerateAssessment = async () => {
    setError(null);
    setIsGenerating(true);
    await generateAssessment({ variables: { websiteId } });
  };

  const handleApprove = async () => {
    setError(null);
    await approveWebsite({ variables: { websiteId } });
  };

  const handleReject = async () => {
    setError(null);
    await rejectWebsite({ variables: { websiteId, reason: 'Rejected by admin' } });
  };

  const handleCrawl = async () => {
    setError(null);
    setIsCrawling(true);
    await crawlWebsite({ variables: { websiteId } });
  };

  const handleSaveMaxPages = async () => {
    setError(null);
    await updateCrawlSettings({
      variables: { websiteId, maxPagesPerCrawl: maxPagesInput },
    });
  };

  const startEditingMaxPages = () => {
    setMaxPagesInput(website?.maxPagesPerCrawl ?? 20);
    setIsEditingMaxPages(true);
  };

  const getListingsForSnapshot = (snapshotUrl: string) => {
    return website?.listings?.filter((listing) => listing.sourceUrl === snapshotUrl) || [];
  };

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'N/A';
    return new Date(dateString).toLocaleString();
  };

  const getStatusBadgeClass = (status: string) => {
    switch (status) {
      case 'approved':
        return 'bg-green-100 text-green-800';
      case 'pending_review':
        return 'bg-amber-100 text-amber-800';
      case 'rejected':
        return 'bg-red-100 text-red-800';
      case 'suspended':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-stone-100 text-stone-800';
    }
  };

  const getRecommendationBadge = (recommendation: string) => {
    switch (recommendation.toLowerCase()) {
      case 'approve':
        return { class: 'bg-green-100 text-green-800 border-green-200', label: 'Recommend Approve' };
      case 'reject':
        return { class: 'bg-red-100 text-red-800 border-red-200', label: 'Recommend Reject' };
      case 'needs_review':
      default:
        return { class: 'bg-amber-100 text-amber-800 border-amber-200', label: 'Needs Review' };
    }
  };

  if (websiteLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading website...</div>
      </div>
    );
  }

  const assessment = assessmentData?.websiteAssessment;

  if (!website) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Website Not Found</h1>
            <Link to="/admin/websites" className="text-blue-600 hover:text-blue-800">
              Back to Websites
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-4xl mx-auto">
        {/* Back Button */}
        <Link
          to="/admin/websites"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          <svg className="w-5 h-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Websites
        </Link>

        {error && (
          <div className="mb-4 p-4 bg-red-50 border border-red-200 text-red-800 rounded-lg">
            {error}
          </div>
        )}

        {/* Website Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start">
            <div>
              <h1 className="text-2xl font-bold text-stone-900 mb-2 select-text">
                <span className="cursor-text">{website.domain}</span>
                <a
                  href={website.domain.startsWith('http') ? website.domain : `https://${website.domain}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="ml-2 text-blue-600 hover:text-blue-800 text-base align-middle"
                  title="Open in new tab"
                >
                  ↗
                </a>
              </h1>
              <div className="flex items-center gap-3 mb-4 select-text">
                <span
                  className={`px-3 py-1 text-sm rounded-full font-medium cursor-text ${getStatusBadgeClass(
                    website.status
                  )}`}
                >
                  {website.status.replace('_', ' ')}
                </span>
                <span className="text-sm text-stone-600 cursor-text">
                  Submitted by: {website.submitterType}
                </span>
              </div>
            </div>

            {/* Action Buttons */}
            <div className="flex gap-2">
              {website.status === 'pending_review' && (
                <>
                  <button
                    onClick={handleApprove}
                    className="bg-green-600 text-white px-4 py-2 rounded-lg hover:bg-green-700"
                  >
                    Approve
                  </button>
                  <button
                    onClick={handleReject}
                    className="bg-red-600 text-white px-4 py-2 rounded-lg hover:bg-red-700"
                  >
                    Reject
                  </button>
                </>
              )}
              <button
                onClick={handleCrawl}
                disabled={isCrawling || website.status !== 'approved'}
                className="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                title={website.status !== 'approved' ? 'Website must be approved to crawl' : 'Crawl multiple pages'}
              >
                {isCrawling ? 'Crawling...' : 'Full Crawl'}
              </button>

              {/* More Menu */}
              <div className="relative">
                <button
                  onClick={() => setShowMoreMenu(!showMoreMenu)}
                  className="bg-stone-200 text-stone-700 px-3 py-2 rounded-lg hover:bg-stone-300"
                  title="More actions"
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                  </svg>
                </button>

                {showMoreMenu && (
                  <>
                    {/* Backdrop to close menu when clicking outside */}
                    <div
                      className="fixed inset-0 z-10"
                      onClick={() => setShowMoreMenu(false)}
                    />
                    <div className="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-stone-200 z-20">
                      <div className="py-1">
                        <button
                          onClick={() => {
                            setShowMoreMenu(false);
                            handleCrawl();
                          }}
                          disabled={isCrawling || website.status !== 'approved'}
                          className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                          </svg>
                          Re-crawl Website
                        </button>
                        <button
                          onClick={() => {
                            setShowMoreMenu(false);
                            // TODO: Implement regenerate page summaries
                            alert('Regenerate page summaries - coming soon');
                          }}
                          disabled={!website.snapshots || website.snapshots.length === 0}
                          className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                          </svg>
                          Regenerate Page Summaries
                        </button>
                        <button
                          onClick={() => {
                            setShowMoreMenu(false);
                            // TODO: Implement regenerate posts
                            alert('Regenerate posts - coming soon');
                          }}
                          disabled={!website.snapshots || website.snapshots.length === 0}
                          className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                          </svg>
                          Regenerate Posts
                        </button>
                        <div className="border-t border-stone-200 my-1" />
                        <button
                          onClick={() => {
                            setShowMoreMenu(false);
                            handleGenerateAssessment();
                          }}
                          disabled={isGenerating}
                          className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                          </svg>
                          {assessment ? 'Regenerate Assessment' : 'Generate Assessment'}
                        </button>
                      </div>
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>

          {/* Website Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-4 pt-4 border-t border-stone-200">
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900 cursor-text">{formatDate(website.createdAt)}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Last Scraped</span>
              <p className="text-sm font-medium text-stone-900 cursor-text">{formatDate(website.lastScrapedAt)}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Snapshots</span>
              <p className="text-sm font-medium text-stone-900 cursor-text">{website.snapshotsCount}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Posts</span>
              <p className="text-sm font-medium text-stone-900 cursor-text">{website.listingsCount}</p>
            </div>
          </div>

          {/* Crawl Status Section */}
          <div className="mt-4 pt-4 border-t border-stone-200">
            <h3 className="text-sm font-semibold text-stone-700 mb-2">Crawl Settings</h3>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Status</span>
                <p className="text-sm font-medium cursor-text">
                  {website.crawlStatus ? (
                    <span
                      className={`px-2 py-0.5 rounded-full text-xs ${
                        website.crawlStatus === 'completed'
                          ? 'bg-green-100 text-green-800'
                          : website.crawlStatus === 'crawling'
                          ? 'bg-blue-100 text-blue-800'
                          : website.crawlStatus === 'no_listings_found'
                          ? 'bg-amber-100 text-amber-800'
                          : website.crawlStatus === 'failed'
                          ? 'bg-red-100 text-red-800'
                          : 'bg-stone-100 text-stone-800'
                      }`}
                    >
                      {website.crawlStatus.replace('_', ' ')}
                    </span>
                  ) : (
                    <span className="text-stone-400 text-xs">Not crawled</span>
                  )}
                </p>
              </div>
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Attempts</span>
                <p className="text-sm font-medium text-stone-900 cursor-text">
                  {website.crawlAttemptCount ?? 0} / {website.maxCrawlRetries ?? 5}
                </p>
              </div>
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase flex items-center gap-1">
                  Max Pages
                  {!isEditingMaxPages && (
                    <button
                      onClick={startEditingMaxPages}
                      className="text-stone-400 hover:text-stone-600"
                      title="Edit max pages"
                    >
                      <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                      </svg>
                    </button>
                  )}
                </span>
                {isEditingMaxPages ? (
                  <div className="flex items-center gap-1 mt-1">
                    <input
                      type="number"
                      min="1"
                      max="100"
                      value={maxPagesInput}
                      onChange={(e) => setMaxPagesInput(parseInt(e.target.value) || 20)}
                      className="w-16 px-2 py-1 text-sm border border-stone-300 rounded"
                    />
                    <button
                      onClick={handleSaveMaxPages}
                      className="px-2 py-1 text-xs bg-blue-600 text-white rounded hover:bg-blue-700"
                    >
                      Save
                    </button>
                    <button
                      onClick={() => setIsEditingMaxPages(false)}
                      className="px-2 py-1 text-xs bg-stone-200 text-stone-700 rounded hover:bg-stone-300"
                    >
                      ✕
                    </button>
                  </div>
                ) : (
                  <p className="text-sm font-medium text-stone-900 cursor-text">
                    {website.pagesCrawledCount ?? 0} / {website.maxPagesPerCrawl ?? 20}
                  </p>
                )}
              </div>
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Last Crawl</span>
                <p className="text-sm font-medium text-stone-900 cursor-text">
                  {formatDate(website.lastCrawlCompletedAt)}
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          {/* Tab Headers */}
          <div className="flex border-b border-stone-200">
            <button
              onClick={() => setActiveTab('listings')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'listings'
                  ? 'border-b-2 border-blue-500 text-blue-600 bg-blue-50/50'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-50'
              }`}
            >
              Posts ({website.listings?.length || 0})
            </button>
            <button
              onClick={() => setActiveTab('snapshots')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'snapshots'
                  ? 'border-b-2 border-blue-500 text-blue-600 bg-blue-50/50'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-50'
              }`}
            >
              Scraped Pages ({website.snapshots?.length || 0})
            </button>
            <button
              onClick={() => setActiveTab('assessment')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'assessment'
                  ? 'border-b-2 border-blue-500 text-blue-600 bg-blue-50/50'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-50'
              }`}
            >
              AI Assessment {assessment ? '✓' : ''}
            </button>
          </div>

          {/* Tab Content */}
          <div className="p-6">
            {/* Listings Tab */}
            {activeTab === 'listings' && (
              <div>
                {website.listings && website.listings.length > 0 ? (
                  <div className="space-y-2">
                    {website.listings.map((listing) => (
                      <div
                        key={listing.id}
                        className="flex items-center justify-between p-3 bg-stone-50 rounded-lg"
                      >
                        <div className="flex-1 min-w-0">
                          <Link
                            to={`/admin/posts/${listing.id}`}
                            className="text-blue-600 hover:text-blue-800 text-sm font-medium"
                          >
                            {listing.title}
                          </Link>
                          <p className="text-xs text-stone-500 mt-1">
                            Created: {formatDate(listing.createdAt)}
                          </p>
                        </div>
                        <div className="ml-4 flex items-center gap-2">
                          <span
                            className={`px-2 py-1 text-xs rounded-full ${
                              listing.status === 'active'
                                ? 'bg-green-100 text-green-800'
                                : listing.status === 'pending_approval'
                                ? 'bg-amber-100 text-amber-800'
                                : listing.status === 'rejected'
                                ? 'bg-red-100 text-red-800'
                                : 'bg-stone-100 text-stone-800'
                            }`}
                          >
                            {listing.status.replace('_', ' ')}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-8 text-stone-500">
                    No posts extracted from this website yet.
                  </div>
                )}
              </div>
            )}

            {/* Scraped Pages Tab */}
            {activeTab === 'snapshots' && (
              <div>
                {website.snapshots && website.snapshots.length > 0 ? (
                  <div className="space-y-2">
                    {website.snapshots.map((snapshot) => {
                      const snapshotListings = getListingsForSnapshot(snapshot.pageUrl);

                      return (
                        <Link
                          key={snapshot.id}
                          to={snapshot.pageSnapshotId ? `/admin/pages/${snapshot.pageSnapshotId}` : '#'}
                          className={`block p-3 border border-stone-200 rounded-lg transition-colors ${
                            snapshot.pageSnapshotId ? 'hover:bg-stone-50 hover:border-stone-300' : 'opacity-60 cursor-not-allowed'
                          }`}
                        >
                          <div className="flex items-center justify-between">
                            <div className="flex-1 min-w-0">
                              <p className="text-sm font-medium text-stone-900 truncate">
                                {snapshot.pageUrl}
                              </p>
                              <div className="flex items-center gap-3 mt-1">
                                <span className="text-xs text-stone-500">
                                  {formatDate(snapshot.lastScrapedAt)}
                                </span>
                                <a
                                  href={snapshot.pageUrl}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="text-xs text-stone-400 hover:text-stone-600 flex items-center gap-0.5"
                                  onClick={(e) => e.stopPropagation()}
                                >
                                  Open original
                                  <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                                  </svg>
                                </a>
                              </div>
                            </div>
                            <div className="ml-4 flex items-center gap-2">
                              <span
                                className={`px-2 py-1 text-xs rounded-full ${
                                  snapshot.scrapeStatus === 'scraped'
                                    ? 'bg-green-100 text-green-800'
                                    : snapshot.scrapeStatus === 'failed'
                                    ? 'bg-red-100 text-red-800'
                                    : 'bg-amber-100 text-amber-800'
                                }`}
                              >
                                {snapshot.scrapeStatus}
                              </span>
                              {snapshotListings.length > 0 && (
                                <span className="px-2 py-1 text-xs rounded-full bg-blue-100 text-blue-800">
                                  {snapshotListings.length} post{snapshotListings.length !== 1 ? 's' : ''}
                                </span>
                              )}
                              <svg className="w-4 h-4 text-stone-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                              </svg>
                            </div>
                          </div>
                        </Link>
                      );
                    })}
                  </div>
                ) : (
                  <div className="text-center py-8 text-stone-500">
                    No pages scraped from this website yet.
                  </div>
                )}
              </div>
            )}

            {/* AI Assessment Tab */}
            {activeTab === 'assessment' && (
              <div>
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-xl font-semibold text-stone-900">AI Assessment</h2>
            <button
              onClick={handleGenerateAssessment}
              disabled={isGenerating}
              className={`px-4 py-2 rounded-lg font-medium ${
                isGenerating
                  ? 'bg-stone-300 text-stone-500 cursor-not-allowed'
                  : assessment
                  ? 'bg-stone-600 text-white hover:bg-stone-700'
                  : 'bg-blue-600 text-white hover:bg-blue-700'
              }`}
            >
              {isGenerating ? (
                <span className="flex items-center">
                  <svg
                    className="animate-spin -ml-1 mr-2 h-4 w-4 text-stone-500"
                    fill="none"
                    viewBox="0 0 24 24"
                  >
                    <circle
                      className="opacity-25"
                      cx="12"
                      cy="12"
                      r="10"
                      stroke="currentColor"
                      strokeWidth="4"
                    />
                    <path
                      className="opacity-75"
                      fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                    />
                  </svg>
                  Generating...
                </span>
              ) : assessment ? (
                'Regenerate Assessment'
              ) : (
                'Generate Assessment'
              )}
            </button>
          </div>

          {assessmentLoading && !assessment && (
            <div className="text-center py-8 text-stone-500">Loading assessment...</div>
          )}

          {!assessment && !assessmentLoading && !isGenerating && (
            <div className="text-center py-12 bg-stone-50 rounded-lg border-2 border-dashed border-stone-300">
              <svg
                className="mx-auto h-12 w-12 text-stone-400"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                />
              </svg>
              <h3 className="mt-2 text-sm font-medium text-stone-900">No assessment yet</h3>
              <p className="mt-1 text-sm text-stone-500">
                Click "Generate Assessment" to create an AI-powered analysis of this website.
              </p>
            </div>
          )}

                {assessment && (
                  <div>
                    {/* Assessment Header */}
                    <div className="flex flex-wrap items-center gap-3 mb-4 pb-4 border-b border-stone-200">
                      <span
                        className={`px-3 py-1 text-sm rounded-full font-medium border ${
                          getRecommendationBadge(assessment.recommendation).class
                        }`}
                      >
                        {getRecommendationBadge(assessment.recommendation).label}
                      </span>

                      {assessment.confidenceScore !== null && (
                        <span className="text-sm text-stone-600">
                          Confidence: {Math.round(assessment.confidenceScore * 100)}%
                        </span>
                      )}

                      {assessment.organizationName && (
                        <span className="text-sm text-stone-600">
                          Org: <strong>{assessment.organizationName}</strong>
                        </span>
                      )}

                      {assessment.foundedYear && (
                        <span className="text-sm text-stone-600">
                          Founded: {assessment.foundedYear}
                        </span>
                      )}
                    </div>

                    {/* Assessment Metadata */}
                    <div className="flex flex-wrap gap-4 text-xs text-stone-500 mb-4">
                      <span>Generated: {formatDate(assessment.generatedAt)}</span>
                      <span>Model: {assessment.modelUsed}</span>
                      {assessment.reviewedByHuman && (
                        <span className="text-green-600">Reviewed by human</span>
                      )}
                    </div>

                    {/* Markdown Content */}
                    <div className="prose prose-stone max-w-none">
                      <ReactMarkdown
                        components={{
                          h1: ({ children }) => (
                            <h1 className="text-xl font-bold text-stone-900 mt-6 mb-3">{children}</h1>
                          ),
                          h2: ({ children }) => (
                            <h2 className="text-lg font-semibold text-stone-800 mt-5 mb-2">{children}</h2>
                          ),
                          h3: ({ children }) => (
                            <h3 className="text-base font-medium text-stone-700 mt-4 mb-2">{children}</h3>
                          ),
                          p: ({ children }) => (
                            <p className="text-stone-700 mb-3 leading-relaxed">{children}</p>
                          ),
                          ul: ({ children }) => (
                            <ul className="list-disc list-inside mb-3 space-y-1">{children}</ul>
                          ),
                          ol: ({ children }) => (
                            <ol className="list-decimal list-inside mb-3 space-y-1">{children}</ol>
                          ),
                          li: ({ children }) => <li className="text-stone-700">{children}</li>,
                          strong: ({ children }) => (
                            <strong className="font-semibold text-stone-900">{children}</strong>
                          ),
                          a: ({ href, children }) => (
                            <a
                              href={href}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="text-blue-600 hover:text-blue-800 underline"
                            >
                              {children}
                            </a>
                          ),
                          blockquote: ({ children }) => (
                            <blockquote className="border-l-4 border-stone-300 pl-4 italic text-stone-600 my-3">
                              {children}
                            </blockquote>
                          ),
                        }}
                      >
                        {assessment.assessmentMarkdown}
                      </ReactMarkdown>
                    </div>
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
