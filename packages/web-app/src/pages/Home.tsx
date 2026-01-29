import { useQuery } from '@apollo/client';
import { Link } from 'react-router-dom';
import { GET_PUBLISHED_POSTS } from '../graphql/queries';
import { PostCard } from '../components/PostCard';

interface Post {
  id: string;
  listingId: string;
  status: string;
  publishedAt?: string;
  expiresAt?: string;
  customTitle?: string;
  customDescription?: string;
  customTldr?: string;
  listing: {
    id: string;
    organizationName: string;
    title: string;
    tldr?: string;
    description: string;
    contactInfo?: {
      email?: string;
      phone?: string;
      website?: string;
    };
    location?: string;
    urgency?: string;
    createdAt: string;
  };
}

export function Home() {
  const { data, loading, error } = useQuery(GET_PUBLISHED_POSTS, {
    variables: { limit: 50 },
  });

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-gray-600">Loading emergency resources...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="text-red-600">Error loading resources: {error.message}</div>
      </div>
    );
  }

  const posts: Post[] = data?.publishedPosts || [];

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white border-b border-gray-200">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-3xl font-bold text-gray-900">
                Emergency Resource Aggregator
              </h1>
              <p className="mt-2 text-gray-600">
                Find help and resources for organizations in need
              </p>
            </div>
            <Link
              to="/submit"
              className="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium shadow-sm flex items-center gap-2"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
              </svg>
              Submit a Resource
            </Link>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {posts.length === 0 ? (
          <div className="text-center py-12">
            <p className="text-gray-500 text-lg">
              No active listings at the moment.
            </p>
          </div>
        ) : (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {posts.map((post) => (
              <PostCard key={post.id} post={post} />
            ))}
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="bg-white border-t border-gray-200 mt-12">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <p className="text-center text-gray-500 text-sm">
            Minnesota Digital Aid - Connecting resources with those who need them
          </p>
        </div>
      </footer>
    </div>
  );
}
