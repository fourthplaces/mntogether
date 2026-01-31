import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation } from '@apollo/client';
import { GET_WEBSITES } from '../../graphql/queries';
import { ADD_ORGANIZATION_SCRAPE_URL, REMOVE_ORGANIZATION_SCRAPE_URL } from '../../graphql/mutations';

interface Website {
  id: string;
  domain: string;
  lastScrapedAt: string | null;
  scrapeFrequencyHours: number;
  active: boolean;
  status: string;
  createdAt: string;
}

export function OrganizationDetail() {
  const { sourceId } = useParams<{ sourceId: string }>();
  const navigate = useNavigate();
  const [newUrl, setNewUrl] = useState('');
  const [error, setError] = useState<string | null>(null);

  const { data, loading, refetch } = useQuery<{ websites: Website[] }>(
    GET_WEBSITES
  );

  const [addUrl] = useMutation(ADD_ORGANIZATION_SCRAPE_URL, {
    onCompleted: () => {
      setNewUrl('');
      setError(null);
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

  const website = data?.websites.find((s) => s.id === sourceId);

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

  if (!website) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-stone-600">Website not found</div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-4xl mx-auto">
        <button
          onClick={() => navigate('/admin/websites')}
          className="mb-6 text-stone-600 hover:text-stone-900 flex items-center gap-2"
        >
          <svg
            className="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          Back to Websites
        </button>

        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h1 className="text-2xl font-bold text-stone-900 mb-2">
            {website.domain}
          </h1>
          <div className="flex gap-4 text-sm text-stone-600">
            <span>Status: {website.status}</span>
            <span>
              Last scraped:{' '}
              {website.lastScrapedAt
                ? new Date(website.lastScrapedAt).toLocaleString()
                : 'Never'}
            </span>
          </div>
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            {error}
          </div>
        )}

        <div className="bg-white rounded-lg shadow-md p-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">
            Website Details
          </h2>

          <div className="space-y-4">
            <div>
              <span className="text-sm text-stone-500">Domain</span>
              <p className="font-medium">{website.domain}</p>
            </div>
            <div>
              <span className="text-sm text-stone-500">Status</span>
              <p className="font-medium">{website.status}</p>
            </div>
            <div>
              <span className="text-sm text-stone-500">Scrape Frequency</span>
              <p className="font-medium">{website.scrapeFrequencyHours} hours</p>
            </div>
            <div>
              <span className="text-sm text-stone-500">Active</span>
              <p className="font-medium">{website.active ? 'Yes' : 'No'}</p>
            </div>
            <div>
              <span className="text-sm text-stone-500">Created</span>
              <p className="font-medium">
                {new Date(website.createdAt).toLocaleString()}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
