import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, gql } from '@apollo/client';
import ReactMarkdown from 'react-markdown';
import { GET_DOMAIN_ASSESSMENT } from '../../graphql/queries';
import { GENERATE_DOMAIN_ASSESSMENT } from '../../graphql/mutations';

const GET_ALL_DOMAINS = gql`
  query GetAllDomains {
    domains(status: null) {
      id
      websiteUrl
      status
      submittedBy
      submitterType
      lastScrapedAt
      snapshotsCount
      listingsCount
      createdAt
    }
  }
`;

const APPROVE_DOMAIN = gql`
  mutation ApproveDomain($domainId: String!) {
    approveDomain(domainId: $domainId) {
      id
      status
    }
  }
`;

const REJECT_DOMAIN = gql`
  mutation RejectDomain($domainId: String!, $reason: String!) {
    rejectDomain(domainId: $domainId, reason: $reason) {
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

interface Website {
  id: string;
  websiteUrl: string;
  status: string;
  submittedBy: string | null;
  submitterType: string;
  lastScrapedAt: string | null;
  snapshotsCount: number;
  listingsCount: number;
  createdAt: string;
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
  const { domainId } = useParams<{ domainId: string }>();
  const [error, setError] = useState<string | null>(null);
  const [isGenerating, setIsGenerating] = useState(false);
  const [isScraping, setIsScraping] = useState(false);

  const { data: domainsData, loading: websiteLoading, refetch: refetchWebsite } = useQuery<{
    domains: Website[];
  }>(GET_ALL_DOMAINS);

  const website = domainsData?.domains.find((d) => d.id === domainId);

  const { data: assessmentData, loading: assessmentLoading, refetch: refetchAssessment } = useQuery<{
    domainAssessment: Assessment | null;
  }>(GET_DOMAIN_ASSESSMENT, {
    variables: { domainId },
    skip: !domainId,
  });

  const [generateAssessment] = useMutation(GENERATE_DOMAIN_ASSESSMENT, {
    onCompleted: () => {
      setIsGenerating(false);
      refetchAssessment();
    },
    onError: (err) => {
      setError(err.message);
      setIsGenerating(false);
    },
  });

  const [approveDomain] = useMutation(APPROVE_DOMAIN, {
    onCompleted: () => refetchWebsite(),
    onError: (err) => setError(err.message),
  });

  const [rejectDomain] = useMutation(REJECT_DOMAIN, {
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

  const handleGenerateAssessment = async () => {
    setError(null);
    setIsGenerating(true);
    await generateAssessment({ variables: { domainId } });
  };

  const handleApprove = async () => {
    setError(null);
    await approveDomain({ variables: { domainId } });
  };

  const handleReject = async () => {
    const reason = prompt('Why are you rejecting this website?');
    if (!reason) return;

    setError(null);
    await rejectDomain({ variables: { domainId, reason } });
  };

  const handleScrape = async () => {
    setError(null);
    setIsScraping(true);
    await scrapeWebsite({ variables: { sourceId: domainId } });
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

  const assessment = assessmentData?.domainAssessment;

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
                  href={`https://${website.websiteUrl}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-blue-600 hover:text-blue-800"
                >
                  {website.websiteUrl}
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
        </div>

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
