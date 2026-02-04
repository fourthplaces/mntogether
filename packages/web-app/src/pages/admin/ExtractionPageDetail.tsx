import { useParams, Link, useSearchParams } from 'react-router-dom';
import { useQuery, gql } from '@apollo/client';
import ReactMarkdown from 'react-markdown';
import { useState } from 'react';

const GET_EXTRACTION_PAGE = gql`
  query GetExtractionPage($url: String!) {
    extractionPage(url: $url) {
      url
      siteUrl
      content
      title
      fetchedAt
      listings {
        id
        title
        status
        createdAt
      }
      listingsCount
    }
  }
`;

interface Listing {
  id: string;
  title: string;
  status: string;
  createdAt: string;
}

interface ExtractionPage {
  url: string;
  siteUrl: string;
  content: string;
  title: string | null;
  fetchedAt: string;
  listings: Listing[];
  listingsCount: number;
}

type TabType = 'posts' | 'content';

export function ExtractionPageDetail() {
  const [searchParams] = useSearchParams();
  const { '*': urlPath } = useParams();

  // Get URL from either search param or path
  const pageUrl = searchParams.get('url') || (urlPath ? decodeURIComponent(urlPath) : null);

  const [activeTab, setActiveTab] = useState<TabType>('content');

  const { data, loading, error } = useQuery<{ extractionPage: ExtractionPage | null }>(GET_EXTRACTION_PAGE, {
    variables: { url: pageUrl },
    skip: !pageUrl,
  });

  const page = data?.extractionPage;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  const getStatusBadgeClass = (status: string) => {
    switch (status) {
      case 'active':
        return 'bg-green-100 text-green-800';
      case 'pending_approval':
        return 'bg-amber-100 text-amber-800';
      case 'completed':
        return 'bg-green-100 text-green-800';
      case 'processing':
        return 'bg-blue-100 text-blue-800';
      case 'pending':
        return 'bg-amber-100 text-amber-800';
      default:
        return 'bg-stone-100 text-stone-800';
    }
  };

  // Extract domain from siteUrl for linking back to website
  const getDomainFromSiteUrl = (siteUrl: string) => {
    try {
      const url = new URL(siteUrl);
      return url.hostname.replace(/^www\./, '');
    } catch {
      return siteUrl;
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading page...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Page</h1>
            <p className="text-stone-600 mb-4">{error.message}</p>
            <Link to="/admin/websites" className="text-blue-600 hover:text-blue-800">
              Back to Websites
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!page) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Page Not Found</h1>
            <p className="text-stone-600 mb-4">URL: {pageUrl}</p>
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
        <button
          onClick={() => window.history.back()}
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          <svg className="w-5 h-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back
        </button>

        {/* Page Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start">
            <div className="flex-1">
              <h1 className="text-2xl font-bold text-stone-900 mb-2 select-text">
                {page.title || 'Extraction Page'}
              </h1>
              <a
                href={page.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-600 hover:text-blue-800 break-all flex items-center gap-1"
              >
                {page.url}
                <svg className="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </a>
              <p className="mt-2 text-sm text-stone-600">
                Site: {getDomainFromSiteUrl(page.siteUrl)}
              </p>
            </div>
          </div>

          {/* Meta Information */}
          <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mt-4 pt-4 border-t border-stone-200">
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Fetched At</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(page.fetchedAt)}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Site URL</span>
              <p className="text-sm font-medium text-stone-900 truncate">{page.siteUrl}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Posts</span>
              <p className="text-sm font-medium text-stone-900">{page.listingsCount}</p>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          {/* Tab Headers */}
          <div className="flex border-b border-stone-200">
            <button
              onClick={() => setActiveTab('content')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'content'
                  ? 'border-b-2 border-blue-500 text-blue-600 bg-blue-50/50'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-50'
              }`}
            >
              Page Content
            </button>
            <button
              onClick={() => setActiveTab('posts')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'posts'
                  ? 'border-b-2 border-blue-500 text-blue-600 bg-blue-50/50'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-50'
              }`}
            >
              Posts ({page.listingsCount})
            </button>
          </div>

          {/* Tab Content */}
          <div className="p-6">
            {/* Page Content Tab */}
            {activeTab === 'content' && (
              <div className="prose prose-stone max-w-none select-text">
                <ReactMarkdown>{page.content}</ReactMarkdown>
              </div>
            )}

            {/* Posts Tab */}
            {activeTab === 'posts' && (
              <div>
                {page.listings.length > 0 ? (
                  <div className="space-y-2">
                    {page.listings.map((listing) => (
                      <Link
                        key={listing.id}
                        to={`/admin/posts/${listing.id}`}
                        className="block p-3 border border-stone-200 rounded-lg hover:bg-stone-50 hover:border-stone-300 transition-colors"
                      >
                        <div className="flex items-center justify-between">
                          <span className="font-medium text-stone-900">{listing.title}</span>
                          <div className="flex items-center gap-2">
                            <span className={`px-2 py-1 text-xs rounded-full ${getStatusBadgeClass(listing.status)}`}>
                              {listing.status.replace('_', ' ')}
                            </span>
                            <svg className="w-4 h-4 text-stone-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                            </svg>
                          </div>
                        </div>
                        <span className="text-xs text-stone-500">{formatDate(listing.createdAt)}</span>
                      </Link>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-8 text-stone-500">
                    No posts extracted from this page yet.
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
