import { useParams, useNavigate } from 'react-router-dom';
import { useQuery } from '@apollo/client';
import { GET_WEBSITES } from '../../graphql/queries';

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

  const { data, loading } = useQuery<{ websites: Website[] }>(
    GET_WEBSITES
  );

  const website = data?.websites.find((s) => s.id === sourceId);

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
