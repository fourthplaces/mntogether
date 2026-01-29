import { useState } from 'react';
import { useQuery, useMutation } from '@apollo/client';
import { GET_ORGANIZATION_SOURCES } from '../graphql/queries';
import { SCRAPE_ORGANIZATION, SUBMIT_RESOURCE_LINK } from '../graphql/mutations';
import { useNavigate } from 'react-router-dom';

interface OrganizationSource {
  id: string;
  organizationName: string;
  sourceUrl: string;
  lastScrapedAt: string | null;
  scrapeFrequencyHours: number;
  active: boolean;
  createdAt: string;
}

export function Resources() {
  const navigate = useNavigate();
  const [showAddForm, setShowAddForm] = useState(false);
  const [newResourceUrl, setNewResourceUrl] = useState('');
  const [newResourceName, setNewResourceName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [scrapingId, setScrapingId] = useState<string | null>(null);

  const { data, loading, refetch } = useQuery<{ organizationSources: OrganizationSource[] }>(
    GET_ORGANIZATION_SOURCES
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
      setNewResourceName('');
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

    if (!newResourceUrl.trim() || !newResourceName.trim()) {
      setError('Please enter both organization name and URL');
      return;
    }

    await submitResourceLink({
      variables: {
        input: {
          url: newResourceUrl,
          context: newResourceName,
        },
      },
    });
  };

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'Never';
    return new Date(dateString).toLocaleString();
  };

  if (loading) {
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
          <h1 className="text-3xl font-bold text-stone-900">Organization Resources</h1>
          <button
            onClick={() => setShowAddForm(!showAddForm)}
            className="bg-amber-700 text-white px-4 py-2 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500"
          >
            {showAddForm ? 'Cancel' : '+ Add Resource'}
          </button>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-800 rounded text-sm">
            {error}
          </div>
        )}

        {showAddForm && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-xl font-semibold text-stone-900 mb-4">Add New Resource</h2>
            <form onSubmit={handleSubmitResource}>
              <div className="mb-4">
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Organization Name
                </label>
                <input
                  type="text"
                  value={newResourceName}
                  onChange={(e) => setNewResourceName(e.target.value)}
                  placeholder="e.g., Red Cross Minnesota"
                  className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                />
              </div>

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
                />
              </div>

              <button
                type="submit"
                className="bg-amber-700 text-white px-4 py-2 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500"
              >
                Submit Resource
              </button>
            </form>
          </div>
        )}

        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          <table className="min-w-full divide-y divide-stone-200">
            <thead className="bg-stone-50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Organization
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Source URL
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Last Scraped
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Status
                </th>
                <th className="px-6 py-3 text-right text-xs font-medium text-stone-700 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="bg-white divide-y divide-stone-200">
              {data?.organizationSources.map((source) => (
                <tr key={source.id} className="hover:bg-stone-50">
                  <td className="px-6 py-4 whitespace-nowrap">
                    <button
                      onClick={() => navigate(`/resources/${source.id}`)}
                      className="text-amber-700 hover:text-amber-900 font-medium"
                    >
                      {source.organizationName}
                    </button>
                  </td>
                  <td className="px-6 py-4">
                    <a
                      href={source.sourceUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-stone-600 hover:text-stone-900 text-sm truncate block max-w-md"
                    >
                      {source.sourceUrl}
                    </a>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm text-stone-600">
                    {formatDate(source.lastScrapedAt)}
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span
                      className={`px-2 py-1 text-xs rounded-full ${
                        source.active
                          ? 'bg-green-100 text-green-800'
                          : 'bg-stone-100 text-stone-800'
                      }`}
                    >
                      {source.active ? 'Active' : 'Inactive'}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-right text-sm">
                    <div className="flex gap-2 justify-end">
                      <button
                        onClick={() => navigate(`/organizations/${source.id}`)}
                        className="bg-blue-600 text-white px-3 py-1 rounded hover:bg-blue-700"
                      >
                        Manage URLs
                      </button>
                      <button
                        onClick={() => handleScrape(source.id)}
                        disabled={scrapingId === source.id}
                        className="bg-amber-600 text-white px-3 py-1 rounded hover:bg-amber-700 disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {scrapingId === source.id ? 'Scraping...' : 'Run Scraper'}
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {data?.organizationSources.length === 0 && (
            <div className="text-center py-12 text-stone-600">
              No resources found. Add a resource to get started.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
