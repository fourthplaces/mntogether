import { useState } from 'react';
import { useQuery, useMutation } from '@apollo/client';
import { GET_ALL_WEBSITES } from '../../graphql/queries';
import { SCRAPE_ORGANIZATION, SUBMIT_RESOURCE_LINK } from '../../graphql/mutations';
import { useNavigate } from 'react-router-dom';
import PaginationControls from '../../components/PaginationControls';
import { useCursorPagination } from '../../hooks/useCursorPagination';

interface Website {
  id: string;
  domain: string;
  status: string;
  lastScrapedAt: string | null;
  snapshotsCount?: number;
  listingsCount?: number;
  submitterType?: string;
  createdAt: string;
}

interface WebsitesResponse {
  websites: {
    nodes: Website[];
    pageInfo: {
      hasNextPage: boolean;
      hasPreviousPage: boolean;
      startCursor: string | null;
      endCursor: string | null;
    };
    totalCount: number;
  };
}

export function Resources() {
  const navigate = useNavigate();
  const [showAddForm, setShowAddForm] = useState(false);
  const [newResourceUrl, setNewResourceUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [scrapingId, setScrapingId] = useState<string | null>(null);

  const pagination = useCursorPagination({ pageSize: 20 });

  const { data, loading, refetch } = useQuery<WebsitesResponse>(
    GET_ALL_WEBSITES,
    {
      variables: {
        ...pagination.variables,
        status: null, // Get all websites
      },
    }
  );

  const websites = data?.websites?.nodes || [];
  const totalCount = data?.websites?.totalCount || 0;
  const pageInfo = data?.websites?.pageInfo || { hasNextPage: false, hasPreviousPage: false };
  const fullPageInfo = pagination.buildPageInfo(
    pageInfo.hasNextPage,
    pageInfo.startCursor,
    pageInfo.endCursor
  );

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
    onError: (err) => {
      setError(err.message);
    },
  });

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
          context: '', // Resources are decoupled from organizations
        },
      },
    });
  };

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'Never';
    return new Date(dateString).toLocaleString();
  };

  if (loading && websites.length === 0) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading resources...</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-6xl mx-auto">
        <div className="flex justify-between items-center mb-6">
          <h1 className="text-3xl font-bold text-stone-900">Websites</h1>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="bg-amber-700 text-white px-4 py-2 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500"
          >
            {showAddForm ? 'Cancel' : '+ Add Website'}
          </button>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-800 rounded text-sm">
            {error}
          </div>
        )}

        {showAddForm && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-xl font-semibold text-stone-900 mb-4">Add New Source URL</h2>
            <form onSubmit={handleSubmitResource}>
              <div className="mb-4">
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Source URL
                </label>
                <input
                  type="url"
                  value={newResourceUrl}
                  onChange={(e) => setNewResourceUrl(e.target.value)}
                  placeholder="https://example.org/needs"
                  className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                  required
                />
                <p className="mt-2 text-sm text-stone-600">
                  Enter the URL of a page that lists emergency resources, services, or opportunities.
                </p>
              </div>

              <button
                type="submit"
                className="bg-amber-700 text-white px-4 py-2 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500"
              >
                Add Source
              </button>
            </form>
          </div>
        )}

        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          <table className="min-w-full divide-y divide-stone-200">
            <thead className="bg-stone-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Website URL
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Last Scraped
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Posts
                </th>
                <th className="px-6 py-3 text-right text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-stone-200">
              {websites.map((website) => (
                <tr key={website.id} className="hover:bg-stone-50">
                  <td className="px-6 py-4">
                    <a
                      href={website.domain}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-amber-700 hover:text-amber-900 font-medium break-all"
                    >
                      {website.domain}
                    </a>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`px-2 py-1 text-xs rounded-full ${
                        website.status === 'approved'
                          ? 'bg-green-100 text-green-800'
                          : website.status === 'pending_review'
                          ? 'bg-yellow-100 text-yellow-800'
                          : 'bg-red-100 text-red-800'
                      }`}
                    >
                      {website.status.replace('_', ' ')}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-600">
                    {formatDate(website.lastScrapedAt)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-600">
                    {website.listingsCount || 0}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-right text-sm">
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => navigate(`/admin/websites/${website.id}`)}
                        className="bg-blue-600 text-white px-3 py-1 rounded hover:bg-blue-700"
                      >
                        View Website
                      </button>
                      <button
                        onClick={() => handleScrape(website.id)}
                        disabled={scrapingId === website.id}
                        className="bg-amber-600 text-white px-3 py-1 rounded hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {scrapingId === website.id ? 'Scraping...' : 'Run Scraper'}
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {websites.length === 0 && (
            <div className="text-center py-12 text-stone-600">
              No websites found. Add a website to get started.
            </div>
          )}
        </div>

        {/* Pagination */}
        {websites.length > 0 && (
          <div className="mt-6">
            <PaginationControls
              pageInfo={fullPageInfo}
              totalCount={totalCount}
              currentPage={pagination.currentPage}
              pageSize={pagination.pageSize}
              onNextPage={() => pagination.goToNextPage(pageInfo.endCursor)}
              onPreviousPage={pagination.goToPreviousPage}
              loading={loading}
            />
          </div>
        )}
      </div>
    </div>
  );
}
