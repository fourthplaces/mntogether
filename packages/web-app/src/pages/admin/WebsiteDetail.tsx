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
      url
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
        scrapeStatus
        scrapeError
        lastScrapedAt
        submittedAt
      }
      listings {
        id
        title
        status
        createdAt
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

const SCRAPE_WEBSITE = gql`
  mutation ScrapeOrganization($sourceId: Uuid!) {
    scrapeOrganization(sourceId: $sourceId) {
      jobId
      status
      message
    }
  }
`;

interface WebsiteSnapshot {
  id: string;
  pageUrl: string;
  scrapeStatus: string;
  scrapeError: string | null;
  lastScrapedAt: string | null;
  submittedAt: string;
}

interface Listing {
  id: string;
  title: string;
  status: string;
  createdAt: string;
}

interface Website {
  id: string;
  url: string;
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

export function WebsiteDetail() {
  const { websiteId } = useParams<{ websiteId: string }>();
  const [error, setError] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isScraping, setIsScraping] = useState(false);
  const [isCrawling, setIsCrawling] = useState(false);

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

  const [scrapeWebsite] = useMutation(SCRAPE_WEBSITE, {
    onCompleted: () => {
      setIsScraping(false);
      refetchWebsite();
    },
    onError: (err) => {
      setError(err.message);
      setIsScraping(false);
    },
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
    const reason = prompt('Why are you rejecting this website?');
    if (!reason) return;

    setError(null);
    await rejectWebsite({ variables: { websiteId, reason } });
  };

  const handleScrape = async () => {
    setError(null);
    setIsScraping(true);
    await scrapeWebsite({ variables: { sourceId: websiteId } });
  };

  const handleCrawl = async () => {
    setError(null);
    setIsCrawling(true);
    await crawlWebsite({ variables: { websiteId } });
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
              <h1 className="text-2xl font-bold text-stone-900 mb-2">
                <a
                  href={`https://${website.url}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-600 hover:text-blue-800"
                >
                  {website.url}
                </a>
              </h1>
              <div className="flex items-center gap-3 mb-4">
                <span
                  className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusBadgeClass(
                    website.status
                  )}`}
                >
                  {website.status.replace('_', ' ')}
                </span>
                <span className="text-sm text-stone-600">
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
                onClick={handleScrape}
                disabled={isScraping}
                className="bg-purple-600 text-white px-4 py-2 rounded-lg hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isScraping ? 'Scraping...' : 'Scrape'}
              </button>
              <button
                onClick={handleCrawl}
                disabled={isCrawling || website.status !== 'approved'}
                className="bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
                title={website.status !== 'approved' ? 'Website must be approved to crawl' : 'Crawl multiple pages'}
              >
                {isCrawling ? 'Crawling...' : 'Full Crawl'}
              </button>
            </div>
          </div>

          {/* Website Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-4 pt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(website.createdAt)}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Last Scraped</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(website.lastScrapedAt)}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Snapshots</span>
              <p className="text-sm font-medium text-stone-900">{website.snapshotsCount}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Listings</span>
              <p className="text-sm font-medium text-stone-900">{website.listingsCount}</p>
            </div>
          </div>

          {/* Crawl Status Section */}
          {website.crawlStatus && (
            <div className="mt-4 pt-4 border-t border-stone-200">
              <h3 className="text-sm font-semibold text-stone-700 mb-2">Crawl Status</h3>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div>
                  <span className="text-xs text-stone-500 uppercase">Status</span>
                  <p className="text-sm font-medium">
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
                  </p>
                </div>
                <div>
                  <span className="text-xs text-stone-500 uppercase">Attempts</span>
                  <p className="text-sm font-medium text-stone-900">
                    {website.crawlAttemptCount ?? 0} / {website.maxCrawlRetries ?? 5}
                  </p>
                </div>
                <div>
                  <span className="text-xs text-stone-500 uppercase">Pages Crawled</span>
                  <p className="text-sm font-medium text-stone-900">
                    {website.pagesCrawledCount ?? 0} / {website.maxPagesPerCrawl ?? 20}
                  </p>
                </div>
                <div>
                  <span className="text-xs text-stone-500 uppercase">Last Crawl</span>
                  <p className="text-sm font-medium text-stone-900">
                    {formatDate(website.lastCrawlCompletedAt)}
                  </p>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Listings Section */}
        {website.listings && website.listings.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-xl font-semibold text-stone-900 mb-4">
              Listings ({website.listings.length})
            </h2>
            <div className="space-y-2">
              {website.listings.map((listing) => (
                <div
                  key={listing.id}
                  className="flex items-center justify-between p-3 bg-stone-50 rounded-lg"
                >
                  <div className="flex-1 min-w-0">
                    <Link
                      to={`/admin/listings/${listing.id}`}
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
          </div>
        )}

        {/* Scraped Pages Section */}
        {website.snapshots && website.snapshots.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-xl font-semibold text-stone-900 mb-4">
              Scraped Pages ({website.snapshots.length})
            </h2>
            <div className="space-y-2">
              {website.snapshots.map((snapshot) => (
                <div
                  key={snapshot.id}
                  className="flex items-center justify-between p-3 bg-stone-50 rounded-lg"
                >
                  <div className="flex-1 min-w-0">
                    <a
                      href={snapshot.pageUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:text-blue-800 text-sm font-medium truncate block"
                    >
                      {snapshot.pageUrl}
                    </a>
                    <p className="text-xs text-stone-500 mt-1">
                      Last scraped: {formatDate(snapshot.lastScrapedAt)}
                    </p>
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
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Assessment Section */}
        <div className="bg-white rounded-lg shadow-md p-6">
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
      </div>
    </div>
  );
}
