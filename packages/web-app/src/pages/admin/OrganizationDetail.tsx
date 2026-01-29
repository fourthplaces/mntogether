import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation } from '@apollo/client';
import { GET_ORGANIZATION_SOURCES } from '../../graphql/queries';
import { ADD_ORGANIZATION_SCRAPE_URL, REMOVE_ORGANIZATION_SCRAPE_URL } from '../../graphql/mutations';

interface OrganizationSource {
  id: string;
  organizationName: string;
  sourceUrl: string;
  scrapeUrls: string[] | null;
  lastScrapedAt: string | null;
  scrapeFrequencyHours: number;
  active: boolean;
  createdAt: string;
}

export function OrganizationDetail() {
  const { sourceId } = useParams<{ sourceId: string }>();
  const navigate = useNavigate();
  const [newUrl, setNewUrl] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [addingUrl, setAddingUrl] = useState(false);

  const { data, loading, refetch } = useQuery<{ organizationSources: OrganizationSource[] }>(
    GET_ORGANIZATION_SOURCES
  );

  const [addUrl] = useMutation(ADD_ORGANIZATION_SCRAPE_URL, {
    onCompleted: () => {
      setNewUrl('');
      setError(null);
      setAddingUrl(false);
      refetch();
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [removeUrl] = useMutation(REMOVE_ORGANIZATION_SCRAPE_URL, {
    onCompleted: () => {
      setError(null);
      refetch();
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const source = data?.organizationSources.find((s) => s.id === sourceId);

  const handleAddUrl = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!newUrl.trim()) {
      setError('Please enter a URL');
      return;
    }

    if (!newUrl.startsWith('http://') && !newUrl.startsWith('https://')) {
      setError('URL must start with http:// or https://');
      return;
    }

    await addUrl({
      variables: {
        sourceId,
        url: newUrl.trim(),
      },
    });
  };

  const handleRemoveUrl = async (url: string) => {
    if (window.confirm(`Remove this URL from scraping?\n\n${url}`)) {
      await removeUrl({
        variables: {
          sourceId,
          url,
        },
      });
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-stone-600">Loading...</div>
      </div>
    );
  }

  if (!source) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-stone-600">Organization not found</div>
      </div>
    );
  }

  const scrapeUrls = source.scrapeUrls || [];
  const hasSpecificUrls = scrapeUrls.length > 0;

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-4xl mx-auto">
        <div className="flex items-center mb-6">
          <button
            onClick={() => navigate('/resources')}
            className="text-amber-700 hover:text-amber-900 mr-4"
          >
            ← Back to Resources
          </button>
          <h1 className="text-3xl font-bold text-stone-900">{source.organizationName}</h1>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-800 rounded text-sm">
            {error}
          </div>
        )}

        {/* Source Info */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-xl font-semibold text-stone-900 mb-4">Organization Source</h2>
          <div className="space-y-2 text-sm">
            <div>
              <span className="font-medium text-stone-700">Base URL:</span>{' '}
              <a
                href={source.sourceUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-amber-700 hover:underline"
              >
                {source.sourceUrl}
              </a>
            </div>
            <div>
              <span className="font-medium text-stone-700">Last Scraped:</span>{' '}
              <span className="text-stone-600">
                {source.lastScrapedAt
                  ? new Date(source.lastScrapedAt).toLocaleString()
                  : 'Never'}
              </span>
            </div>
            <div>
              <span className="font-medium text-stone-700">Scrape Frequency:</span>{' '}
              <span className="text-stone-600">Every {source.scrapeFrequencyHours} hours</span>
            </div>
          </div>
        </div>

        {/* Scraping Strategy Info */}
        <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
          <h3 className="text-sm font-semibold text-blue-900 mb-2">
            {hasSpecificUrls ? '🎯 Targeted Scraping' : '🌐 Site-Wide Crawling'}
          </h3>
          <p className="text-sm text-blue-800">
            {hasSpecificUrls
              ? `Scraping ${scrapeUrls.length} specific URL${scrapeUrls.length !== 1 ? 's' : ''}. Only these pages will be processed.`
              : 'No specific URLs configured. The entire site will be crawled when scraping (up to 15 pages).'}
          </p>
        </div>

        {/* Scrape URLs Management */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-xl font-semibold text-stone-900 mb-4">Specific URLs to Scrape</h2>

          {/* Add URL Form */}
          {!addingUrl ? (
            <button
              onClick={() => setAddingUrl(true)}
              className="mb-4 bg-amber-700 text-white px-4 py-2 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500"
            >
              + Add URL
            </button>
          ) : (
            <form onSubmit={handleAddUrl} className="mb-6">
              <div className="flex gap-2">
                <input
                  type="url"
                  value={newUrl}
                  onChange={(e) => setNewUrl(e.target.value)}
                  placeholder="https://example.com/volunteer-opportunities"
                  className="flex-1 px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                  autoFocus
                />
                <button
                  type="submit"
                  className="bg-green-600 text-white px-4 py-2 rounded-md hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500"
                >
                  Add
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setAddingUrl(false);
                    setNewUrl('');
                    setError(null);
                  }}
                  className="bg-stone-100 text-stone-700 px-4 py-2 rounded-md hover:bg-stone-200 focus:outline-none focus:ring-2 focus:ring-stone-500"
                >
                  Cancel
                </button>
              </div>
            </form>
          )}

          {/* URL List */}
          {scrapeUrls.length > 0 ? (
            <div className="space-y-2">
              {scrapeUrls.map((url, index) => (
                <div
                  key={index}
                  className="flex items-center justify-between p-3 bg-stone-50 rounded border border-stone-200"
                >
                  <a
                    href={url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm text-amber-700 hover:underline flex-1 mr-4 break-all"
                  >
                    {url}
                  </a>
                  <button
                    onClick={() => handleRemoveUrl(url)}
                    className="bg-red-600 text-white px-3 py-1 rounded text-sm hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 flex-shrink-0"
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-stone-600 bg-stone-50 rounded-lg border border-stone-200">
              <p className="mb-2">No specific URLs configured</p>
              <p className="text-sm">Add URLs to scrape specific pages, or leave empty to crawl the entire site.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
