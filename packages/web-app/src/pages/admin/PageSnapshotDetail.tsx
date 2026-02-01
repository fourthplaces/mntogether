import { useParams, Link } from 'react-router-dom';
import { useQuery, gql } from '@apollo/client';
import ReactMarkdown from 'react-markdown';
import { useState } from 'react';

const GET_PAGE_SNAPSHOT = gql`
  query GetPageSnapshot($id: Uuid!) {
    pageSnapshot(id: $id) {
      id
      url
      markdown
      html
      fetchedVia
      crawledAt
      extractionStatus
      listingsExtractedCount
      summary
      listings {
        id
        title
        status
        createdAt
      }
    }
  }
`;

interface Listing {
  id: string;
  title: string;
  status: string;
  createdAt: string;
}

interface PageSnapshot {
  id: string;
  url: string;
  markdown: string | null;
  html: string;
  fetchedVia: string;
  crawledAt: string;
  extractionStatus: string | null;
  listingsExtractedCount: number | null;
  summary: string | null;
  listings: Listing[];
}

type TabType = 'posts' | 'summary' | 'content';

export function PageSnapshotDetail() {
  const { snapshotId } = useParams<{ snapshotId: string }>();
  const [activeTab, setActiveTab] = useState<TabType>('posts');
  const [contentMode, setContentMode] = useState<'markdown' | 'html'>('markdown');
  const [showMoreMenu, setShowMoreMenu] = useState(false);

  const { data, loading, error } = useQuery<{ pageSnapshot: PageSnapshot | null }>(GET_PAGE_SNAPSHOT, {
    variables: { id: snapshotId },
    skip: !snapshotId,
  });

  const snapshot = data?.pageSnapshot;

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

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading page snapshot...</div>
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

  if (!snapshot) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Page Not Found</h1>
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
                Scraped Page
              </h1>
              <a
                href={snapshot.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-600 hover:text-blue-800 break-all flex items-center gap-1"
              >
                {snapshot.url}
                <svg className="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
              </a>
            </div>

            {/* More Menu */}
            <div className="relative ml-4">
              <button
                onClick={() => setShowMoreMenu(!showMoreMenu)}
                className="bg-stone-200 text-stone-700 px-3 py-2 rounded-lg hover:bg-stone-300"
                title="More actions"
              >
                <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                </svg>
              </button>

              {showMoreMenu && (
                <>
                  <div
                    className="fixed inset-0 z-10"
                    onClick={() => setShowMoreMenu(false)}
                  />
                  <div className="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-stone-200 z-20">
                    <div className="py-1">
                      <button
                        onClick={() => {
                          setShowMoreMenu(false);
                          // TODO: Implement regenerate summary
                          alert('Regenerate AI summary - coming soon');
                        }}
                        className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 flex items-center gap-2"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                        Regenerate AI Summary
                      </button>
                      <button
                        onClick={() => {
                          setShowMoreMenu(false);
                          // TODO: Implement regenerate posts
                          alert('Regenerate posts from this page - coming soon');
                        }}
                        className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 flex items-center gap-2"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                        </svg>
                        Regenerate Posts
                      </button>
                      <div className="border-t border-stone-200 my-1" />
                      <button
                        onClick={() => {
                          setShowMoreMenu(false);
                          // TODO: Implement re-scrape
                          alert('Re-scrape page - coming soon');
                        }}
                        className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 flex items-center gap-2"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                        </svg>
                        Re-scrape Page
                      </button>
                    </div>
                  </div>
                </>
              )}
            </div>
          </div>

          {/* Meta Information */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mt-4 pt-4 border-t border-stone-200">
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Crawled At</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(snapshot.crawledAt)}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Fetched Via</span>
              <p className="text-sm font-medium text-stone-900">{snapshot.fetchedVia}</p>
            </div>
            {snapshot.extractionStatus && (
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Extraction</span>
                <p className="text-sm font-medium">
                  <span className={`px-2 py-1 text-xs rounded-full ${getStatusBadgeClass(snapshot.extractionStatus)}`}>
                    {snapshot.extractionStatus}
                  </span>
                </p>
              </div>
            )}
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Posts</span>
              <p className="text-sm font-medium text-stone-900">{snapshot.listings.length}</p>
            </div>
          </div>
        </div>

        {/* Tabs */}
        <div className="bg-white rounded-lg shadow-md overflow-hidden">
          {/* Tab Headers */}
          <div className="flex border-b border-stone-200">
            <button
              onClick={() => setActiveTab('posts')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'posts'
                  ? 'border-b-2 border-blue-500 text-blue-600 bg-blue-50/50'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-50'
              }`}
            >
              Posts ({snapshot.listings.length})
            </button>
            <button
              onClick={() => setActiveTab('summary')}
              className={`px-6 py-3 font-medium text-sm ${
                activeTab === 'summary'
                  ? 'border-b-2 border-blue-500 text-blue-600 bg-blue-50/50'
                  : 'text-stone-600 hover:text-stone-900 hover:bg-stone-50'
              }`}
            >
              AI Summary {snapshot.summary ? '' : '(none)'}
            </button>
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
          </div>

          {/* Tab Content */}
          <div className="p-6">
            {/* Posts Tab */}
            {activeTab === 'posts' && (
              <div>
                {snapshot.listings.length > 0 ? (
                  <div className="space-y-2">
                    {snapshot.listings.map((listing) => (
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

            {/* AI Summary Tab */}
            {activeTab === 'summary' && (
              <div>
                {snapshot.summary ? (
                  <div className="prose prose-stone max-w-none select-text">
                    <ReactMarkdown>{snapshot.summary}</ReactMarkdown>
                  </div>
                ) : (
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
                    <h3 className="mt-2 text-sm font-medium text-stone-900">No AI summary</h3>
                    <p className="mt-1 text-sm text-stone-500">
                      Use the menu to regenerate the AI summary.
                    </p>
                  </div>
                )}
              </div>
            )}

            {/* Page Content Tab */}
            {activeTab === 'content' && (
              <div>
                {/* Content Mode Toggle */}
                <div className="flex gap-2 mb-4">
                  {snapshot.markdown && (
                    <button
                      onClick={() => setContentMode('markdown')}
                      className={`px-3 py-1.5 rounded text-sm font-medium transition-colors ${
                        contentMode === 'markdown'
                          ? 'bg-stone-800 text-white'
                          : 'bg-stone-100 text-stone-700 hover:bg-stone-200'
                      }`}
                    >
                      Rendered
                    </button>
                  )}
                  <button
                    onClick={() => setContentMode('html')}
                    className={`px-3 py-1.5 rounded text-sm font-medium transition-colors ${
                      contentMode === 'html'
                        ? 'bg-stone-800 text-white'
                        : 'bg-stone-100 text-stone-700 hover:bg-stone-200'
                    }`}
                  >
                    Raw HTML
                  </button>
                </div>

                {contentMode === 'markdown' && snapshot.markdown ? (
                  <div className="prose prose-stone max-w-none select-text">
                    <ReactMarkdown>{snapshot.markdown}</ReactMarkdown>
                  </div>
                ) : (
                  <div className="font-mono text-sm text-stone-700 whitespace-pre-wrap break-all max-h-[600px] overflow-y-auto bg-stone-50 p-4 rounded-lg select-text">
                    {snapshot.html}
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
