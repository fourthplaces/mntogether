import { useState, useEffect } from 'react';
import { useQuery, useMutation, useLazyQuery, gql } from '@apollo/client';
import { useNavigate, useSearchParams } from 'react-router-dom';

const GET_ALL_WEBSITES = gql`
  query GetAllWebsites($agentId: String) {
    websites(status: null, agentId: $agentId) {
      id
      domain
      status
      submitterType
      lastScrapedAt
      snapshotsCount
      listingsCount
      agentId
      createdAt
      crawlStatus
      crawlAttemptCount
      pagesCrawledCount
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

const SCRAPE_ORGANIZATION = gql`
  mutation ScrapeOrganization($sourceId: Uuid!) {
    scrapeOrganization(sourceId: $sourceId) {
      jobId
      status
    }
  }
`;

const CRAWL_WEBSITE = gql`
  mutation CrawlWebsite($websiteId: Uuid!) {
    crawlWebsite(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

const SUBMIT_RESOURCE_LINK = gql`
  mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
    submitResourceLink(input: $input) {
      status
      message
    }
  }
`;

const SEARCH_WEBSITES = gql`
  query SearchWebsites($query: String!, $limit: Int, $threshold: Float) {
    searchWebsites(query: $query, limit: $limit, threshold: $threshold) {
      websiteId
      assessmentId
      websiteDomain
      organizationName
      recommendation
      assessmentMarkdown
      similarity
    }
  }
`;

interface WebsiteSearchResult {
  websiteId: string;
  assessmentId: string;
  websiteDomain: string;
  organizationName: string | null;
  recommendation: string;
  assessmentMarkdown: string;
  similarity: number;
}

interface Website {
  id: string;
  domain: string;
  status: string;
  submitterType: string;
  lastScrapedAt: string | null;
  snapshotsCount: number;
  listingsCount: number;
  createdAt: string;
  agentId: string | null;
  crawlStatus: string | null;
  crawlAttemptCount: number | null;
  pagesCrawledCount: number | null;
}

export function Websites() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const agentIdFilter = searchParams.get('agentId');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [showAddForm, setShowAddForm] = useState(false);
  const [newResourceUrl, setNewResourceUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [scrapingId, setScrapingId] = useState<string | null>(null);
  const [selectedWebsites, setSelectedWebsites] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  const [showSearchResults, setShowSearchResults] = useState(false);
  const [crawlingId, setCrawlingId] = useState<string | null>(null);

  const { data, loading, refetch } = useQuery<{ websites: Website[] }>(GET_ALL_WEBSITES, {
    variables: { agentId: agentIdFilter },
  });

  const clearAgentFilter = () => {
    setSearchParams({});
  };

  const [executeSearch, { data: searchData, loading: searchLoading }] = useLazyQuery<{
    searchWebsites: WebsiteSearchResult[];
  }>(SEARCH_WEBSITES);

  const [approveWebsite] = useMutation(APPROVE_WEBSITE, {
    onCompleted: () => refetch(),
    onError: (err) => setError(err.message),
  });

  const [rejectWebsite] = useMutation(REJECT_WEBSITE, {
    onCompleted: () => refetch(),
    onError: (err) => setError(err.message),
  });

  const [scrapeOrganization] = useMutation(SCRAPE_ORGANIZATION, {
    onCompleted: () => {
      setScrapingId(null);
      refetch();
    },
    onError: (err) => {
      setError(err.message);
      setScrapingId(null);
    },
  });

  const [crawlWebsite] = useMutation(CRAWL_WEBSITE, {
    onCompleted: () => {
      setCrawlingId(null);
      refetch();
    },
    onError: (err) => {
      setError(err.message);
      setCrawlingId(null);
    },
  });

  const [submitResourceLink] = useMutation(SUBMIT_RESOURCE_LINK, {
    onCompleted: () => {
      setShowAddForm(false);
      setNewResourceUrl('');
      setError(null);
      refetch();
    },
    onError: (err) => setError(err.message),
  });

  const handleApprove = async (websiteId: string) => {
    setError(null);
    await approveWebsite({ variables: { websiteId } });
  };

  const handleReject = async (websiteId: string) => {
    setError(null);
    await rejectWebsite({ variables: { websiteId, reason: 'Rejected by admin' } });
  };

  const handleScrape = async (sourceId: string) => {
    setScrapingId(sourceId);
    setError(null);
    await scrapeOrganization({ variables: { sourceId } });
  };

  const handleCrawl = async (websiteId: string) => {
    setCrawlingId(websiteId);
    setError(null);
    await crawlWebsite({ variables: { websiteId } });
  };

  const handleSubmitResource = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!newResourceUrl.trim()) {
      setError('Please enter a source URL');
      return;
    }

    await submitResourceLink({
      variables: {
        input: {
          url: newResourceUrl,
          context: '',
        },
      },
    });
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchQuery.trim()) return;

    setError(null);
    setShowSearchResults(true);
    await executeSearch({
      variables: {
        query: searchQuery,
        limit: 20,
        threshold: 0.4,
      },
    });
  };

  const clearSearch = () => {
    setSearchQuery('');
    setShowSearchResults(false);
  };

  const toggleWebsiteSelection = (websiteId: string) => {
    const newSelection = new Set(selectedWebsites);
    if (newSelection.has(websiteId)) {
      newSelection.delete(websiteId);
    } else {
      newSelection.add(websiteId);
    }
    setSelectedWebsites(newSelection);
  };

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'Never';
    return new Date(dateString).toLocaleString();
  };

  // Filter websites
  const filteredWebsites = data?.websites.filter((website) => {
    if (statusFilter === 'all') return true;
    return website.status === statusFilter;
  });

  const pendingCount = data?.websites.filter((d) => d.status === 'pending_review').length || 0;
  const approvedCount = data?.websites.filter((d) => d.status === 'approved').length || 0;
  const rejectedCount = data?.websites.filter((d) => d.status === 'rejected').length || 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading websites...</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex justify-between items-start mb-6">
          <div>
            <h1 className="text-3xl font-bold text-stone-900 mb-2">Website Management</h1>
            <p className="text-stone-600">
              Approve websites for scraping, monitor extraction, and manage content sources
            </p>
          </div>
          <button
            onClick={() => setShowAddForm(true)}
            className="bg-blue-600 text-white px-6 py-3 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 font-medium"
          >
            + Add Website
          </button>
        </div>

        {error && (
          <div className="mb-4 p-4 bg-red-50 border border-red-200 text-red-800 rounded-lg">
            {error}
          </div>
        )}

        {/* Agent Filter Indicator */}
        {agentIdFilter && (
          <div className="mb-4 p-4 bg-purple-50 border border-purple-200 rounded-lg flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-purple-700 font-medium">Filtered by Agent</span>
              <span className="text-purple-600 text-sm">
                Showing websites discovered by agent: {agentIdFilter.slice(0, 8)}...
              </span>
            </div>
            <button
              onClick={clearAgentFilter}
              className="text-purple-700 hover:text-purple-900 font-medium text-sm"
            >
              Clear Filter
            </button>
          </div>
        )}

        {/* Semantic Search */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-3">Search Websites</h2>
          <form onSubmit={handleSearch} className="flex gap-3">
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="e.g., find me a law firm helping immigrants, food shelves in Minneapolis..."
              className="flex-1 px-4 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
            <button
              type="submit"
              disabled={searchLoading || !searchQuery.trim()}
              className="bg-blue-600 text-white px-6 py-2 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {searchLoading ? 'Searching...' : 'Search'}
            </button>
            {showSearchResults && (
              <button
                type="button"
                onClick={clearSearch}
                className="bg-stone-600 text-white px-4 py-2 rounded-md hover:bg-stone-700"
              >
                Clear
              </button>
            )}
          </form>
          <p className="mt-2 text-sm text-stone-500">
            Search using natural language to find websites semantically
          </p>
        </div>

        {/* Search Results */}
        {showSearchResults && (
          <div className="bg-white rounded-lg shadow-md mb-6">
            <div className="p-4 border-b border-stone-200">
              <h3 className="text-lg font-semibold text-stone-900">
                Search Results for "{searchQuery}"
              </h3>
            </div>
            {searchLoading ? (
              <div className="p-8 text-center text-stone-600">Searching...</div>
            ) : searchData?.searchWebsites && searchData.searchWebsites.length > 0 ? (
              <div className="divide-y divide-stone-200">
                {searchData.searchWebsites.map((result) => (
                  <div
                    key={result.assessmentId}
                    className="p-4 hover:bg-stone-50 cursor-pointer"
                    onClick={() => navigate(`/admin/websites/${result.websiteId}`)}
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <a
                            href={result.websiteDomain.startsWith('http') ? result.websiteDomain : `https://${result.websiteDomain}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-blue-600 hover:text-blue-800 font-medium"
                            onClick={(e) => e.stopPropagation()}
                          >
                            {result.websiteDomain}
                          </a>
                          <span
                            className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                              result.recommendation === 'approve'
                                ? 'bg-green-100 text-green-800'
                                : result.recommendation === 'reject'
                                ? 'bg-red-100 text-red-800'
                                : 'bg-amber-100 text-amber-800'
                            }`}
                          >
                            {result.recommendation}
                          </span>
                        </div>
                        {result.organizationName && (
                          <p className="text-sm text-stone-700 font-medium">
                            {result.organizationName}
                          </p>
                        )}
                        <p className="text-sm text-stone-600 mt-1 line-clamp-2">
                          {result.assessmentMarkdown.slice(0, 200)}...
                        </p>
                      </div>
                      <div className="ml-4 text-right">
                        <div className="text-sm font-medium text-stone-900">
                          {(result.similarity * 100).toFixed(0)}% match
                        </div>
                        <div className="text-xs text-stone-500">similarity</div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="p-8 text-center text-stone-600">
                No websites found matching your query. Try a different search term.
              </div>
            )}
          </div>
        )}


        {/* Status Filter Tabs */}
        <div className="bg-white rounded-lg shadow-md mb-6">
          <div className="flex border-b border-stone-200">
            <button
              onClick={() => setStatusFilter('all')}
              className={`px-6 py-3 font-medium ${
                statusFilter === 'all'
                  ? 'border-b-2 border-blue-500 text-blue-600'
                  : 'text-stone-600 hover:text-stone-900'
              }`}
            >
              All ({data?.websites.length || 0})
            </button>
            <button
              onClick={() => setStatusFilter('pending_review')}
              className={`px-6 py-3 font-medium ${
                statusFilter === 'pending_review'
                  ? 'border-b-2 border-amber-500 text-amber-600'
                  : 'text-stone-600 hover:text-stone-900'
              }`}
            >
              Pending ({pendingCount})
            </button>
            <button
              onClick={() => setStatusFilter('approved')}
              className={`px-6 py-3 font-medium ${
                statusFilter === 'approved'
                  ? 'border-b-2 border-green-500 text-green-600'
                  : 'text-stone-600 hover:text-stone-900'
              }`}
            >
              Approved ({approvedCount})
            </button>
            <button
              onClick={() => setStatusFilter('rejected')}
              className={`px-6 py-3 font-medium ${
                statusFilter === 'rejected'
                  ? 'border-b-2 border-red-500 text-red-600'
                  : 'text-stone-600 hover:text-stone-900'
              }`}
            >
              Rejected ({rejectedCount})
            </button>
          </div>
        </div>

        {/* Bulk Actions */}
        {selectedWebsites.size > 0 && (
          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-blue-900">
                {selectedWebsites.size} website(s) selected
              </span>
              <div className="flex gap-2">
                <button className="bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700 text-sm">
                  Approve Selected
                </button>
                <button className="bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700 text-sm">
                  Reject Selected
                </button>
                <button
                  onClick={() => setSelectedWebsites(new Set())}
                  className="bg-stone-600 text-white px-4 py-2 rounded hover:bg-stone-700 text-sm"
                >
                  Clear
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Websites - Mobile Cards / Desktop Table */}
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          {/* Desktop Table - Hidden on mobile */}
          <div className="hidden lg:block overflow-x-auto">
            <table className="min-w-full divide-y divide-stone-200">
              <thead className="bg-stone-50">
                <tr>
                  <th className="px-4 py-3 text-left">
                    <input
                      type="checkbox"
                      onChange={(e) => {
                        if (e.target.checked) {
                          setSelectedWebsites(
                            new Set(filteredWebsites?.map((d) => d.id) || [])
                          );
                        } else {
                          setSelectedWebsites(new Set());
                        }
                      }}
                      className="rounded"
                    />
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                    Website
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                    Status
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                    Listings
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                    Crawl
                  </th>
                  <th className="px-4 py-3 text-right text-xs font-medium text-stone-700 uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="bg-white divide-y divide-stone-200">
                {filteredWebsites?.map((website) => (
                  <tr
                    key={website.id}
                    className="hover:bg-stone-50 cursor-pointer"
                    onClick={() => navigate(`/admin/websites/${website.id}`)}
                  >
                    <td className="px-4 py-4" onClick={(e) => e.stopPropagation()}>
                      <input
                        type="checkbox"
                        checked={selectedWebsites.has(website.id)}
                        onChange={() => toggleWebsiteSelection(website.id)}
                        className="rounded"
                      />
                    </td>
                    <td className="px-4 py-4 max-w-xs">
                      <div className="flex items-center gap-1">
                        <span className="text-stone-900 font-medium text-sm break-all line-clamp-2 select-text cursor-text">
                          {website.domain}
                        </span>
                        <a
                          href={website.domain.startsWith('http') ? website.domain : `https://${website.domain}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-blue-600 hover:text-blue-800 flex-shrink-0"
                          onClick={(e) => e.stopPropagation()}
                          title="Open website"
                        >
                          ↗
                        </a>
                      </div>
                      {website.agentId && (
                        <span className="text-xs px-1.5 py-0.5 bg-purple-100 text-purple-700 rounded mt-1 inline-block">
                          Agent
                        </span>
                      )}
                    </td>
                    <td className="px-4 py-4 whitespace-nowrap">
                      <span
                        className={`px-2 py-1 text-xs rounded-full font-medium ${
                          website.status === 'approved'
                            ? 'bg-green-100 text-green-800'
                            : website.status === 'pending_review'
                            ? 'bg-amber-100 text-amber-800'
                            : 'bg-red-100 text-red-800'
                        }`}
                      >
                        {website.status === 'pending_review' ? 'pending' : website.status}
                      </span>
                    </td>
                    <td className="px-4 py-4 whitespace-nowrap">
                      <span className="text-sm font-semibold text-stone-900">
                        {website.listingsCount || 0}
                      </span>
                    </td>
                    <td className="px-4 py-4 whitespace-nowrap">
                      {website.crawlStatus ? (
                        <span
                          className={`px-2 py-0.5 text-xs rounded-full ${
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
                          {website.crawlStatus === 'no_listings_found' ? 'no listings' : website.crawlStatus}
                        </span>
                      ) : (
                        <span className="text-xs text-stone-400">-</span>
                      )}
                    </td>
                    <td className="px-4 py-4 whitespace-nowrap text-right text-sm" onClick={(e) => e.stopPropagation()}>
                      <div className="flex gap-1 justify-end flex-wrap">
                        {website.status === 'pending_review' && (
                          <>
                            <button
                              onClick={() => handleApprove(website.id)}
                              className="bg-green-600 text-white px-2 py-1 rounded text-xs hover:bg-green-700"
                            >
                              Approve
                            </button>
                            <button
                              onClick={() => handleReject(website.id)}
                              className="bg-red-600 text-white px-2 py-1 rounded text-xs hover:bg-red-700"
                            >
                              Reject
                            </button>
                          </>
                        )}
                        {website.status === 'approved' && (
                          <button
                            onClick={() => handleCrawl(website.id)}
                            disabled={crawlingId === website.id}
                            className="bg-indigo-600 text-white px-2 py-1 rounded text-xs hover:bg-indigo-700 disabled:opacity-50"
                          >
                            {crawlingId === website.id ? '...' : 'Crawl'}
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Mobile Cards - Shown only on mobile/tablet */}
          <div className="lg:hidden divide-y divide-stone-200">
            {filteredWebsites?.map((website) => (
              <div
                key={website.id}
                className="p-4 cursor-pointer hover:bg-stone-50"
                onClick={() => navigate(`/admin/websites/${website.id}`)}
              >
                <div className="flex items-start justify-between gap-2 mb-2">
                  <div className="flex-1 min-w-0 flex items-center gap-1">
                    <span className="text-stone-900 font-medium text-sm break-all select-text cursor-text">
                      {website.domain}
                    </span>
                    <a
                      href={website.domain.startsWith('http') ? website.domain : `https://${website.domain}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:text-blue-800 flex-shrink-0"
                      onClick={(e) => e.stopPropagation()}
                      title="Open website"
                    >
                      ↗
                    </a>
                  </div>
                  <span
                    className={`px-2 py-1 text-xs rounded-full font-medium whitespace-nowrap ${
                      website.status === 'approved'
                        ? 'bg-green-100 text-green-800'
                        : website.status === 'pending_review'
                        ? 'bg-amber-100 text-amber-800'
                        : 'bg-red-100 text-red-800'
                    }`}
                  >
                    {website.status === 'pending_review' ? 'pending' : website.status}
                  </span>
                </div>

                <div className="flex flex-wrap gap-2 text-xs text-stone-600 mb-3">
                  <span>Listings: <strong className="text-stone-900">{website.listingsCount || 0}</strong></span>
                  {website.crawlStatus && (
                    <span
                      className={`px-1.5 py-0.5 rounded ${
                        website.crawlStatus === 'completed'
                          ? 'bg-green-100 text-green-800'
                          : website.crawlStatus === 'crawling'
                          ? 'bg-blue-100 text-blue-800'
                          : 'bg-stone-100 text-stone-800'
                      }`}
                    >
                      {website.crawlStatus}
                    </span>
                  )}
                  {website.agentId && (
                    <span className="px-1.5 py-0.5 bg-purple-100 text-purple-700 rounded">
                      Agent
                    </span>
                  )}
                </div>

                <div className="flex flex-wrap gap-2" onClick={(e) => e.stopPropagation()}>
                  {website.status === 'pending_review' && (
                    <>
                      <button
                        onClick={() => handleApprove(website.id)}
                        className="bg-green-600 text-white px-3 py-1.5 rounded text-sm hover:bg-green-700"
                      >
                        Approve
                      </button>
                      <button
                        onClick={() => handleReject(website.id)}
                        className="bg-red-600 text-white px-3 py-1.5 rounded text-sm hover:bg-red-700"
                      >
                        Reject
                      </button>
                    </>
                  )}
                  {website.status === 'approved' && (
                    <button
                      onClick={() => handleCrawl(website.id)}
                      disabled={crawlingId === website.id}
                      className="bg-indigo-600 text-white px-3 py-1.5 rounded text-sm hover:bg-indigo-700 disabled:opacity-50"
                    >
                      {crawlingId === website.id ? 'Crawling...' : 'Crawl'}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>

          {filteredWebsites?.length === 0 && (
            <div className="text-center py-12 text-stone-600">
              No websites found with status: {statusFilter}
            </div>
          )}
        </div>
      </div>

      {/* Add Website Modal */}
      {showAddForm && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-6">
          <div className="bg-white rounded-lg shadow-xl max-w-md w-full">
            <div className="p-6">
              <div className="flex justify-between items-center mb-4">
                <h2 className="text-xl font-semibold text-stone-900">Add New Website</h2>
                <button
                  onClick={() => setShowAddForm(false)}
                  className="text-stone-400 hover:text-stone-600 text-2xl"
                >
                  &times;
                </button>
              </div>
              <form onSubmit={handleSubmitResource}>
                <div className="mb-4">
                  <label className="block text-sm font-medium text-stone-700 mb-2">
                    Website URL
                  </label>
                  <input
                    type="url"
                    value={newResourceUrl}
                    onChange={(e) => setNewResourceUrl(e.target.value)}
                    placeholder="https://example.org/resources"
                    className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                    autoFocus
                    required
                  />
                  <p className="mt-2 text-sm text-stone-600">
                    Enter the URL of a page that lists community resources, services, or volunteer
                    opportunities.
                  </p>
                </div>

                <div className="flex gap-3">
                  <button
                    type="submit"
                    className="flex-1 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 font-medium"
                  >
                    Add Website
                  </button>
                  <button
                    type="button"
                    onClick={() => setShowAddForm(false)}
                    className="px-4 py-2 border border-stone-300 rounded-md hover:bg-stone-50 font-medium"
                  >
                    Cancel
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
