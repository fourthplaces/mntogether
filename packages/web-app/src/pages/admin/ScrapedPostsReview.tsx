import React, { useState, useEffect } from 'react';
import { useQuery, useMutation } from '@apollo/client';
import { GET_SCRAPED_PENDING_POSTS, GET_SCRAPED_POSTS_STATS } from '../../graphql/queries';
import { gql } from '@apollo/client';
import PostReviewCard from '../../components/PostReviewCard';
import PostEditModal from '../../components/PostEditModal';
import PaginationControls from '../../components/PaginationControls';
import { useCursorPagination } from '../../hooks/useCursorPagination';

const APPROVE_POST = gql`
  mutation ApprovePost($listingId: Uuid!) {
    approveListing(listingId: $listingId) {
      id
      status
    }
  }
`;

const REJECT_POST = gql`
  mutation RejectPost($listingId: Uuid!, $reason: String) {
    rejectListing(listingId: $listingId, reason: $reason)
  }
`;

type PostType = 'all' | 'service' | 'opportunity' | 'business';

const ScrapedPostsReview: React.FC = () => {
  const [selectedType, setSelectedType] = useState<PostType>('all');
  const [editingPost, setEditingPost] = useState<any>(null);

  const pagination = useCursorPagination({ pageSize: 10 });

  // Reset pagination when filter changes
  useEffect(() => {
    pagination.reset();
  }, [selectedType]);

  // Fetch stats
  const { data: statsData } = useQuery(GET_SCRAPED_POSTS_STATS);

  // Fetch posts with cursor pagination
  const { data, loading, error, refetch } = useQuery(GET_SCRAPED_PENDING_POSTS, {
    variables: {
      postType: selectedType === 'all' ? null : selectedType,
      ...pagination.variables,
    },
    fetchPolicy: 'network-only',
  });

  const [approvePost] = useMutation(APPROVE_POST, {
    onCompleted: () => {
      refetch();
    },
  });

  const [rejectPost] = useMutation(REJECT_POST, {
    onCompleted: () => {
      refetch();
    },
  });

  const handleApprove = async (postId: string) => {
    if (confirm('Are you sure you want to approve this post?')) {
      try {
        await approvePost({ variables: { listingId: postId } });
      } catch (err) {
        console.error('Failed to approve post:', err);
        alert('Failed to approve post. Check console for details.');
      }
    }
  };

  const handleReject = async (postId: string, reason?: string) => {
    try {
      await rejectPost({
        variables: {
          listingId: postId,
          reason: reason || null,
        },
      });
    } catch (err) {
      console.error('Failed to reject post:', err);
      alert('Failed to reject post. Check console for details.');
    }
  };

  const handleEdit = (post: any) => {
    setEditingPost(post);
  };

  const handleEditSuccess = () => {
    refetch();
  };

  const posts = data?.listings?.nodes || [];
  const totalCount = data?.listings?.totalCount || 0;
  const pageInfo = data?.listings?.pageInfo || { hasNextPage: false, hasPreviousPage: false };
  const fullPageInfo = pagination.buildPageInfo(
    pageInfo.hasNextPage,
    pageInfo.startCursor,
    pageInfo.endCursor
  );

  const stats = {
    services: statsData?.scrapedPendingServices?.totalCount || 0,
    opportunities: statsData?.scrapedPendingOpportunities?.totalCount || 0,
    businesses: statsData?.scrapedPendingBusinesses?.totalCount || 0,
  };

  const totalPending = stats.services + stats.opportunities + stats.businesses;

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-3xl font-bold text-stone-900 mb-2">
            Scraped Posts Review
          </h1>
          <p className="text-stone-600">
            Review and approve posts extracted by the intelligent crawler
          </p>
        </div>

        {/* Stats Dashboard */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
          <div
            className={`bg-white border-2 rounded-lg p-4 cursor-pointer transition-all ${
              selectedType === 'all'
                ? 'border-amber-500 shadow-md'
                : 'border-stone-200 hover:border-stone-300'
            }`}
            onClick={() => setSelectedType('all')}
          >
            <div className="text-2xl font-bold text-stone-900">{totalPending}</div>
            <div className="text-sm text-stone-600">Total Pending</div>
          </div>

          <div
            className={`bg-white border-2 rounded-lg p-4 cursor-pointer transition-all ${
              selectedType === 'service'
                ? 'border-blue-500 shadow-md'
                : 'border-stone-200 hover:border-stone-300'
            }`}
            onClick={() => setSelectedType('service')}
          >
            <div className="text-2xl font-bold text-blue-700">{stats.services}</div>
            <div className="text-sm text-stone-600">Services</div>
          </div>

          <div
            className={`bg-white border-2 rounded-lg p-4 cursor-pointer transition-all ${
              selectedType === 'opportunity'
                ? 'border-green-500 shadow-md'
                : 'border-stone-200 hover:border-stone-300'
            }`}
            onClick={() => setSelectedType('opportunity')}
          >
            <div className="text-2xl font-bold text-green-700">{stats.opportunities}</div>
            <div className="text-sm text-stone-600">Opportunities</div>
          </div>

          <div
            className={`bg-white border-2 rounded-lg p-4 cursor-pointer transition-all ${
              selectedType === 'business'
                ? 'border-purple-500 shadow-md'
                : 'border-stone-200 hover:border-stone-300'
            }`}
            onClick={() => setSelectedType('business')}
          >
            <div className="text-2xl font-bold text-purple-700">{stats.businesses}</div>
            <div className="text-sm text-stone-600">Businesses</div>
          </div>
        </div>

        {/* Active Filter Badge */}
        {selectedType !== 'all' && (
          <div className="mb-4">
            <span className="inline-flex items-center gap-2 px-3 py-1 bg-amber-100 text-amber-800 rounded-full text-sm">
              Filtering: <span className="font-semibold capitalize">{selectedType}</span>
              <button
                onClick={() => setSelectedType('all')}
                className="hover:text-amber-900"
              >
                ✕
              </button>
            </span>
          </div>
        )}

        {/* Loading State */}
        {loading && (
          <div className="text-center py-12">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-amber-600"></div>
            <p className="mt-2 text-stone-600">Loading posts...</p>
          </div>
        )}

        {/* Error State */}
        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-6">
            <strong>Error:</strong> {error.message}
          </div>
        )}

        {/* Empty State */}
        {!loading && !error && posts.length === 0 && (
          <div className="bg-white border border-stone-200 rounded-lg p-12 text-center">
            <div className="text-6xl mb-4">🎉</div>
            <h3 className="text-xl font-semibold text-stone-900 mb-2">
              All caught up!
            </h3>
            <p className="text-stone-600">
              No pending {selectedType !== 'all' ? selectedType : ''} posts to review.
            </p>
          </div>
        )}

        {/* Posts Grid */}
        {!loading && !error && posts.length > 0 && (
          <>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">
              {posts.map((post: any) => (
                <PostReviewCard
                  key={post.id}
                  listing={post}
                  onApprove={handleApprove}
                  onReject={handleReject}
                  onEdit={handleEdit}
                />
              ))}
            </div>

            {/* Pagination */}
            <PaginationControls
              pageInfo={fullPageInfo}
              totalCount={totalCount}
              currentPage={pagination.currentPage}
              pageSize={pagination.pageSize}
              onNextPage={() => pagination.goToNextPage(pageInfo.endCursor)}
              onPreviousPage={pagination.goToPreviousPage}
              loading={loading}
            />
          </>
        )}

        {/* Helpful Tips */}
        <div className="mt-6 bg-white border border-amber-200 rounded-lg p-6">
          <h3 className="font-semibold text-amber-900 mb-3 flex items-center gap-2">
            Review Tips
          </h3>
          <ul className="text-sm text-stone-700 space-y-2 list-disc list-inside">
            <li>
              <strong>Services</strong> offer help (legal aid, healthcare, food pantries)
            </li>
            <li>
              <strong>Opportunities</strong> need help (volunteers, donations, partnerships)
            </li>
            <li>
              <strong>Businesses</strong> donate proceeds to causes
            </li>
            <li>Click "Show more" to see full details and type-specific fields</li>
            <li>Use "Edit" to improve text before approving</li>
            <li>Check source URL to verify accuracy</li>
            <li>Reject spam, duplicates, or irrelevant content</li>
          </ul>
        </div>
      </div>

      {/* Edit Modal */}
      {editingPost && (
        <PostEditModal
          listing={editingPost}
          onClose={() => setEditingPost(null)}
          onSuccess={handleEditSuccess}
        />
      )}
    </div>
  );
};

export default ScrapedPostsReview;
