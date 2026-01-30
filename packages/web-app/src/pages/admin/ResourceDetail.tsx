import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation } from '@apollo/client';
import {
  GET_ORGANIZATION_SOURCE_LISTINGS,
  GET_ORGANIZATION_SOURCES,
  GET_POSTS_FOR_LISTING,
} from '../../graphql/queries';
import {
  APPROVE_LISTING,
  REJECT_LISTING,
  ARCHIVE_POST,
  EXPIRE_POST,
  DELETE_LISTING,
} from '../../graphql/mutations';

interface Listing {
  id: string;
  organizationName: string;
  title: string;
  tldr: string;
  description: string;
  status: string;
  submissionType: string;
  sourceUrl?: string;
  createdAt: string;
}

interface Post {
  id: string;
  status: string;
  expiresAt: string | null;
  createdAt: string;
}

export function ResourceDetail() {
  const { sourceId } = useParams<{ sourceId: string }>();
  const navigate = useNavigate();
  const [selectedListing, setSelectedListing] = useState<string | null>(null);
  const [rejectReason, setRejectReason] = useState('');
  const [showRejectModal, setShowRejectModal] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sourceUrl, setSourceUrl] = useState<string>('');

  // Get source URL from organization sources
  const { data: sourcesData } = useQuery(GET_ORGANIZATION_SOURCES);

  useEffect(() => {
    if (sourcesData?.organizationSources) {
      const source = sourcesData.organizationSources.find((s: any) => s.id === sourceId);
      if (source) {
        setSourceUrl(source.sourceUrl);
      }
    }
  }, [sourcesData, sourceId]);

  const { data: listingsData, loading, refetch } = useQuery<{ listings: { nodes: Listing[] } }>(
    GET_ORGANIZATION_SOURCE_LISTINGS,
    {
      variables: {
        status: 'PENDING_APPROVAL', // Scraped listings start as pending approval
      },
      skip: !sourceId,
    }
  );

  const [approveListing] = useMutation(APPROVE_LISTING, {
    onCompleted: () => {
      refetch();
      setError(null);
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [rejectListing] = useMutation(REJECT_LISTING, {
    onCompleted: () => {
      setShowRejectModal(false);
      setRejectReason('');
      refetch();
      setError(null);
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [archivePost] = useMutation(ARCHIVE_POST, {
    onCompleted: () => {
      refetch();
      setError(null);
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [_expirePost] = useMutation(EXPIRE_POST, {
    onCompleted: () => {
      refetch();
      setError(null);
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [deleteListing] = useMutation(DELETE_LISTING, {
    onCompleted: () => {
      refetch();
      setError(null);
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const handleApprove = async (listingId: string) => {
    await approveListing({ variables: { listingId } });
  };

  const handleReject = async (listingId: string) => {
    setSelectedListing(listingId);
    setShowRejectModal(true);
  };

  const confirmReject = async () => {
    if (!selectedListing || !rejectReason.trim()) {
      setError('Please provide a rejection reason');
      return;
    }
    await rejectListing({ variables: { listingId: selectedListing, reason: rejectReason } });
  };

  const handleUnpublish = async (postId: string) => {
    if (window.confirm('Are you sure you want to unpublish this listing?')) {
      await archivePost({ variables: { postId } });
    }
  };

  const handleDelete = async (listingId: string) => {
    if (window.confirm('Are you sure you want to delete this listing? This action cannot be undone.')) {
      await deleteListing({ variables: { listingId } });
    }
  };

  const getStatusColor = (status: string) => {
    switch (status.toLowerCase()) {
      case 'active':
        return 'bg-green-100 text-green-800';
      case 'pending_approval':
        return 'bg-yellow-100 text-yellow-800';
      case 'rejected':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-stone-100 text-stone-800';
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-stone-600">Loading listings...</div>
      </div>
    );
  }

  // Get all listings for this source (no filtering by organization name)
  const filteredListings = listingsData?.listings.nodes || [];

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-6xl mx-auto">
        <div className="flex flex-col mb-6">
          <button
            onClick={() => navigate('/admin/resources')}
            className="text-amber-700 hover:text-amber-900 mb-2 self-start"
          >
            ← Back to Resources
          </button>
          <h1 className="text-2xl font-bold text-stone-900 break-all">{sourceUrl}</h1>
          <p className="text-sm text-stone-600 mt-1">Listings scraped from this source</p>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-800 rounded text-sm">
            {error}
          </div>
        )}

        <div className="space-y-4">
          {filteredListings.map((listing) => (
            <ListingCard
              key={listing.id}
              listing={listing}
              onApprove={handleApprove}
              onReject={handleReject}
              onUnpublish={handleUnpublish}
              onDelete={handleDelete}
              getStatusColor={getStatusColor}
            />
          ))}

          {filteredListings.length === 0 && (
            <div className="bg-white rounded-lg shadow-md p-12 text-center text-stone-600">
              No listings found for this organization. Run the scraper to fetch data.
            </div>
          )}
        </div>

        {/* Reject Modal */}
        {showRejectModal && (
          <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
            <div className="bg-white rounded-lg shadow-xl p-6 max-w-md w-full">
              <h2 className="text-xl font-semibold text-stone-900 mb-4">Reject Listing</h2>
              <div className="mb-4">
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Rejection Reason
                </label>
                <textarea
                  value={rejectReason}
                  onChange={(e) => setRejectReason(e.target.value)}
                  placeholder="Explain why this listing is being rejected..."
                  className="w-full px-3 py-2 border border-stone-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                  rows={4}
                />
              </div>
              <div className="flex gap-2">
                <button
                  onClick={confirmReject}
                  className="flex-1 bg-red-600 text-white px-4 py-2 rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500"
                >
                  Confirm Reject
                </button>
                <button
                  onClick={() => {
                    setShowRejectModal(false);
                    setRejectReason('');
                  }}
                  className="flex-1 bg-stone-100 text-stone-700 px-4 py-2 rounded-md hover:bg-stone-200 focus:outline-none focus:ring-2 focus:ring-stone-500"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function ListingCard({
  listing,
  onApprove,
  onReject,
  onUnpublish,
  onDelete,
  getStatusColor,
}: {
  listing: Listing;
  onApprove: (id: string) => void;
  onReject: (id: string) => void;
  onUnpublish: (postId: string) => void;
  onDelete: (id: string) => void;
  getStatusColor: (status: string) => string;
}) {
  const { data: postsData } = useQuery<{ postsForListing: Post[] }>(GET_POSTS_FOR_LISTING, {
    variables: { listingId: listing.id },
  });

  const activePosts = postsData?.postsForListing.filter(
    (post) => post.status === 'published' || post.status === 'active'
  ) || [];

  const isPending = listing.status.toLowerCase() === 'pending_approval';
  const isActive = listing.status.toLowerCase() === 'active';

  return (
    <div className="bg-white rounded-lg shadow-md p-6">
      <div className="flex justify-between items-start mb-4">
        <div className="flex-1">
          <h3 className="text-lg font-semibold text-stone-900 mb-2">{listing.title}</h3>
          <p className="text-sm text-stone-600 mb-2">{listing.tldr}</p>
          {listing.sourceUrl && (
            <a
              href={listing.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-amber-700 hover:text-amber-900 hover:underline inline-flex items-center gap-1 mb-2"
            >
              View source page ↗
            </a>
          )}
          <div>
            <span className={`inline-block px-2 py-1 text-xs rounded-full ${getStatusColor(listing.status)}`}>
              {listing.status}
            </span>
          </div>
        </div>
      </div>

      {listing.description && (
        <div className="mb-4 p-3 bg-stone-50 rounded text-sm text-stone-700">
          {listing.description}
        </div>
      )}

      <div className="flex items-center justify-between pt-4 border-t border-stone-200">
        <div className="text-sm text-stone-600">
          {activePosts.length > 0 ? (
            <span className="text-green-600 font-medium">
              ✓ Published ({activePosts.length} post{activePosts.length !== 1 ? 's' : ''})
            </span>
          ) : (
            <span className="text-stone-500">Not published</span>
          )}
        </div>

        <div className="flex gap-2">
          {isPending && (
            <>
              <button
                onClick={() => onApprove(listing.id)}
                className="bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700 text-sm"
              >
                ✓ Approve
              </button>
              <button
                onClick={() => onReject(listing.id)}
                className="bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700 text-sm"
              >
                ✗ Deny
              </button>
            </>
          )}

          {isActive && activePosts.length > 0 && (
            <button
              onClick={() => onUnpublish(activePosts[0].id)}
              className="bg-orange-600 text-white px-4 py-2 rounded hover:bg-orange-700 text-sm"
            >
              Unpublish
            </button>
          )}

          <button
            onClick={() => onDelete(listing.id)}
            className="bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700 text-sm"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}
