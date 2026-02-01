import { useQuery, useMutation } from '@apollo/client';
import { GET_PENDING_POSTS } from '../../graphql/queries';
import { APPROVE_POST, REJECT_POST } from '../../graphql/mutations';
import { useState } from 'react';

interface Post {
  id: string;
  organizationName: string;
  title: string;
  tldr: string;
  description: string;
  contactInfo?: {
    email?: string;
    phone?: string;
    website?: string;
  };
  urgency?: string;
  location?: string;
  submissionType?: string;
  createdAt: string;
}

export function PostApprovalQueue() {
  const [selectedPost, setSelectedPost] = useState<Post | null>(null);
  const { data, loading, error, refetch } = useQuery(GET_PENDING_POSTS, {
    variables: { limit: 50, offset: 0 },
  });

  const [approvePost] = useMutation(APPROVE_POST, {
    onCompleted: () => {
      refetch();
      setSelectedPost(null);
    },
  });

  const [rejectPost] = useMutation(REJECT_POST, {
    onCompleted: () => {
      refetch();
      setSelectedPost(null);
    },
  });

  if (loading) return <div className="p-8">Loading...</div>;
  if (error) return <div className="p-8 text-red-600">Error: {error.message}</div>;

  const posts = data?.listings?.nodes || [];

  const handleApprove = async (postId: string) => {
    if (confirm('Approve this post? It will become visible to all volunteers.')) {
      await approvePost({ variables: { listingId: postId } });
    }
  };

  const handleReject = async (postId: string) => {
    const reason = prompt('Reason for rejection (optional):');
    if (reason !== null) {
      await rejectPost({ variables: { listingId: postId, reason: reason || 'Rejected by admin' } });
    }
  };

  return (
    <div className="max-w-7xl mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">Post Approval Queue</h1>

      {posts.length === 0 ? (
        <div className="text-stone-500 text-center py-12">
          No pending posts to review
        </div>
      ) : (
        <div className="grid gap-6">
          {posts.map((post: Post) => (
            <div
              key={post.id}
              className="bg-white border border-stone-200 rounded-lg p-6 hover:shadow-lg transition-shadow"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-xs font-medium px-2 py-1 bg-stone-100 rounded">
                      {post.submissionType === 'user_submitted' ? '👤 User' : '🌐 Scraped'}
                    </span>
                    {post.urgency && (
                      <span className={`text-xs font-medium px-2 py-1 rounded ${
                        post.urgency === 'urgent' ? 'bg-red-100 text-red-700' :
                        post.urgency === 'low' ? 'bg-amber-100 text-amber-700' :
                        'bg-yellow-100 text-yellow-700'
                      }`}>
                        {post.urgency}
                      </span>
                    )}
                  </div>
                  <h3 className="text-xl font-semibold mb-1">{post.title}</h3>
                  <p className="text-sm text-stone-600 mb-2">{post.organizationName}</p>
                  {post.location && (
                    <p className="text-sm text-stone-500 mb-2">📍 {post.location}</p>
                  )}
                  <p className="text-stone-700 mb-4">{post.tldr}</p>
                </div>
              </div>

              <div className="flex gap-2">
                <button
                  onClick={() => setSelectedPost(post)}
                  className="px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700"
                >
                  View Details
                </button>
                <button
                  onClick={() => handleApprove(post.id)}
                  className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
                >
                  ✓ Approve
                </button>
                <button
                  onClick={() => handleReject(post.id)}
                  className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
                >
                  ✗ Reject
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Detail Modal */}
      {selectedPost && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4">
          <div className="bg-white rounded-lg max-w-2xl w-full max-h-[80vh] overflow-y-auto p-6">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-2xl font-bold">{selectedPost.title}</h2>
              <button
                onClick={() => setSelectedPost(null)}
                className="text-stone-500 hover:text-stone-700"
              >
                ✕
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <h3 className="font-semibold text-stone-700">Organization</h3>
                <p>{selectedPost.organizationName}</p>
              </div>

              {selectedPost.location && (
                <div>
                  <h3 className="font-semibold text-stone-700">Location</h3>
                  <p>{selectedPost.location}</p>
                </div>
              )}

              <div>
                <h3 className="font-semibold text-stone-700">Description</h3>
                <p className="whitespace-pre-wrap">{selectedPost.description}</p>
              </div>

              {selectedPost.contactInfo && (
                <div>
                  <h3 className="font-semibold text-stone-700">Contact</h3>
                  {selectedPost.contactInfo.email && <p>Email: {selectedPost.contactInfo.email}</p>}
                  {selectedPost.contactInfo.phone && <p>Phone: {selectedPost.contactInfo.phone}</p>}
                  {selectedPost.contactInfo.website && (
                    <p>
                      Website:{' '}
                      <a
                        href={selectedPost.contactInfo.website}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-amber-600 hover:underline"
                      >
                        {selectedPost.contactInfo.website}
                      </a>
                    </p>
                  )}
                </div>
              )}
            </div>

            <div className="flex gap-2 mt-6">
              <button
                onClick={() => handleApprove(selectedPost.id)}
                className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
              >
                ✓ Approve
              </button>
              <button
                onClick={() => handleReject(selectedPost.id)}
                className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
              >
                ✗ Reject
              </button>
              <button
                onClick={() => setSelectedPost(null)}
                className="px-4 py-2 bg-stone-300 text-stone-700 rounded hover:bg-stone-400"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
