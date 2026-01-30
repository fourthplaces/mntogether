import { useState } from 'react';
import { useQuery, useMutation, gql } from '@apollo/client';
import { useNavigate } from 'react-router-dom';

const GET_ALL_DOMAINS = gql`
  query GetAllDomains {
    domains(status: null) {
      id
      domainUrl
      status
      submitterType
      lastScrapedAt
      snapshotsCount
      listingsCount
      createdAt
      agentId
      tavilyRelevanceScore
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

const SCRAPE_ORGANIZATION = gql`
  mutation ScrapeOrganization($sourceId: Uuid!) {
    scrapeOrganization(sourceId: $sourceId) {
      jobId
      status
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

interface Domain {
  id: string;
  domainUrl: string;
  status: string;
  submitterType: string;
  lastScrapedAt: string | null;
  snapshotsCount: number;
  listingsCount: number;
  createdAt: string;
  agentId: string | null;
  tavilyRelevanceScore: number | null;
}

export function Domains() {
  const navigate = useNavigate();
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [showAddForm, setShowAddForm] = useState(false);
  const [newResourceUrl, setNewResourceUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [scrapingId, setScrapingId] = useState<string | null>(null);
  const [selectedDomains, setSelectedDomains] = useState<Set<string>>(new Set());

  const { data, loading, refetch } = useQuery<{ domains: Domain[] }>(GET_ALL_DOMAINS);

  const [approveDomain] = useMutation(APPROVE_DOMAIN, {
    onCompleted: () => refetch(),
    onError: (err) => setError(err.message),
  });

  const [rejectDomain] = useMutation(REJECT_DOMAIN, {
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

  const [submitResourceLink] = useMutation(SUBMIT_RESOURCE_LINK, {
    onCompleted: () => {
      setShowAddForm(false);
      setNewResourceUrl('');
      setError(null);
      refetch();
    },
    onError: (err) => setError(err.message),
  });

  const handleApprove = async (domainId: string) => {
    setError(null);
    await approveDomain({ variables: { domainId } });
  };

  const handleReject = async (domainId: string) => {
    const reason = prompt('Why are you rejecting this domain?');
    if (!reason) return;

    setError(null);
    await rejectDomain({ variables: { domainId, reason } });
  };

  const handleScrape = async (sourceId: string) => {
    setScrapingId(sourceId);
    setError(null);
    await scrapeOrganization({ variables: { sourceId } });
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

  const toggleDomainSelection = (domainId: string) => {
    const newSelection = new Set(selectedDomains);
    if (newSelection.has(domainId)) {
      newSelection.delete(domainId);
    } else {
      newSelection.add(domainId);
    }
    setSelectedDomains(newSelection);
  };

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'Never';
    return new Date(dateString).toLocaleString();
  };

  // Filter domains
  const filteredDomains = data?.domains.filter((domain) => {
    if (statusFilter === 'all') return true;
    return domain.status === statusFilter;
  });

  const pendingCount = data?.domains.filter((d) => d.status === 'pending_review').length || 0;
  const approvedCount = data?.domains.filter((d) => d.status === 'approved').length || 0;
  const rejectedCount = data?.domains.filter((d) => d.status === 'rejected').length || 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading domains...</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stone-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex justify-between items-start mb-6">
          <div>
            <h1 className="text-3xl font-bold text-stone-900 mb-2">Domain Management</h1>
            <p className="text-stone-600">
              Approve domains for scraping, monitor extraction, and manage content sources
            </p>
          </div>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="bg-blue-600 text-white px-6 py-3 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 font-medium"
          >
            {showAddForm ? 'Cancel' : '+ Add Domain'}
          </button>
        </div>

        {error && (
          <div className="mb-4 p-4 bg-red-50 border border-red-200 text-red-800 rounded-lg">
            {error}
          </div>
        )}

        {/* Add Domain Form */}
        {showAddForm && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-xl font-semibold text-stone-900 mb-4">Add New Domain</h2>
            <form onSubmit={handleSubmitResource}>
              <div className="mb-4">
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Source URL
                </label>
                <input
                  type="url"
                  value={newResourceUrl}
                  onChange={(e) => setNewResourceUrl(e.target.value)}
                  placeholder="https://example.org/resources"
                  className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  required
                />
                <p className="mt-2 text-sm text-stone-600">
                  Enter the URL of a page that lists community resources, services, or volunteer
                  opportunities.
                </p>
              </div>

              <button
                type="submit"
                className="bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                Add Domain
              </button>
            </form>
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
              All ({data?.domains.length || 0})
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
        {selectedDomains.size > 0 && (
          <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-blue-900">
                {selectedDomains.size} domain(s) selected
              </span>
              <div className="flex gap-2">
                <button className="bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700 text-sm">
                  Approve Selected
                </button>
                <button className="bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700 text-sm">
                  Reject Selected
                </button>
                <button
                  onClick={() => setSelectedDomains(new Set())}
                  className="bg-stone-600 text-white px-4 py-2 rounded hover:bg-stone-700 text-sm"
                >
                  Clear
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Domains Table */}
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          <table className="min-w-full divide-y divide-stone-200">
            <thead className="bg-stone-50">
              <tr>
                <th className="px-4 py-3 text-left">
                  <input
                    type="checkbox"
                    onChange={(e) => {
                      if (e.target.checked) {
                        setSelectedDomains(
                          new Set(filteredDomains?.map((d) => d.id) || [])
                        );
                      } else {
                        setSelectedDomains(new Set());
                      }
                    }}
                    className="rounded"
                  />
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Domain
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Source
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Last Scraped
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Listings
                </th>
                <th className="px-6 py-3 text-right text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-stone-200">
              {filteredDomains?.map((domain) => (
                <tr key={domain.id} className="hover:bg-stone-50">
                  <td className="px-4 py-4">
                    <input
                      type="checkbox"
                      checked={selectedDomains.has(domain.id)}
                      onChange={() => toggleDomainSelection(domain.id)}
                      className="rounded"
                    />
                  </td>
                  <td className="px-6 py-4">
                    <a
                      href={`https://${domain.domainUrl}`}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:text-blue-800 font-medium break-all"
                    >
                      {domain.domainUrl}
                    </a>
                    {domain.agentId && (
                      <div className="mt-1">
                        <span className="text-xs px-2 py-1 bg-purple-100 text-purple-700 rounded">
                          🤖 Discovered by Agent
                        </span>
                      </div>
                    )}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`px-2 py-1 text-xs rounded-full font-medium ${
                        domain.status === 'approved'
                          ? 'bg-green-100 text-green-800'
                          : domain.status === 'pending_review'
                          ? 'bg-amber-100 text-amber-800'
                          : 'bg-red-100 text-red-800'
                      }`}
                    >
                      {domain.status.replace('_', ' ')}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className="text-sm text-stone-600">{domain.submitterType}</span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-600">
                    {formatDate(domain.lastScrapedAt)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className="text-sm font-semibold text-stone-900">
                      {domain.listingsCount || 0}
                    </span>
                    {domain.snapshotsCount > 0 && (
                      <span className="text-xs text-stone-500 ml-2">
                        ({domain.snapshotsCount} snapshots)
                      </span>
                    )}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-right text-sm">
                    <div className="flex gap-2 justify-end">
                      {domain.status === 'pending_review' && (
                        <>
                          <button
                            onClick={() => handleApprove(domain.id)}
                            className="bg-green-600 text-white px-3 py-1 rounded hover:bg-green-700"
                          >
                            Approve
                          </button>
                          <button
                            onClick={() => handleReject(domain.id)}
                            className="bg-red-600 text-white px-3 py-1 rounded hover:bg-red-700"
                          >
                            Reject
                          </button>
                        </>
                      )}
                      <button
                        onClick={() => navigate(`/admin/domains/${domain.id}`)}
                        className="bg-blue-600 text-white px-3 py-1 rounded hover:bg-blue-700"
                      >
                        View
                      </button>
                      {domain.status === 'approved' && (
                        <button
                          onClick={() => handleScrape(domain.id)}
                          disabled={scrapingId === domain.id}
                          className="bg-purple-600 text-white px-3 py-1 rounded hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                          {scrapingId === domain.id ? 'Scraping...' : 'Scrape'}
                        </button>
                      )}
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {filteredDomains?.length === 0 && (
            <div className="text-center py-12 text-stone-600">
              No domains found with status: {statusFilter}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
