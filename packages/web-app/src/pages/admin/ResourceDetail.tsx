import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation } from '@apollo/client';
import {
  GET_ORGANIZATION_SOURCE_NEEDS,
  GET_ORGANIZATION_SOURCES,
  GET_POSTS_FOR_NEED,
} from '../../graphql/queries';
import {
  APPROVE_NEED,
  REJECT_NEED,
  ARCHIVE_POST,
  EXPIRE_POST,
  DELETE_NEED,
} from '../../graphql/mutations';

interface Need {
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
  const [selectedNeed, setSelectedNeed] = useState<string | null>(null);
  const [rejectReason, setRejectReason] = useState('');
  const [showRejectModal, setShowRejectModal] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [organizationName, setOrganizationName] = useState<string>('');

  // Get organization source to find the name
  const { data: sourcesData } = useQuery(GET_ORGANIZATION_SOURCES);

  useEffect(() => {
    if (sourcesData?.organizationSources) {
      const source = sourcesData.organizationSources.find((s: any) => s.id === sourceId);
      if (source) {
        setOrganizationName(source.organizationName);
      }
    }
  }, [sourcesData, sourceId]);

  const { data: needsData, loading, refetch } = useQuery<{ needs: { nodes: Need[] } }>(
    GET_ORGANIZATION_SOURCE_NEEDS,
    {
      variables: {
        status: 'PENDING_APPROVAL', // Scraped needs start as pending approval
      },
      skip: !organizationName,
    }
  );

  const [approveNeed] = useMutation(APPROVE_NEED, {
    onCompleted: () => {
      refetch();
      setError(null);
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const [rejectNeed] = useMutation(REJECT_NEED, {
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

  const [deleteNeed] = useMutation(DELETE_NEED, {
    onCompleted: () => {
      refetch();
      setError(null);
    },
    onError: (err) => {
      setError(err.message);
    },
  });

  const handleApprove = async (needId: string) => {
    await approveNeed({ variables: { needId } });
  };

  const handleReject = async (needId: string) => {
    setSelectedNeed(needId);
    setShowRejectModal(true);
  };

  const confirmReject = async () => {
    if (!selectedNeed || !rejectReason.trim()) {
      setError('Please provide a rejection reason');
      return;
    }
    await rejectNeed({ variables: { needId: selectedNeed, reason: rejectReason } });
  };

  const handleUnpublish = async (postId: string) => {
    if (window.confirm('Are you sure you want to unpublish this need?')) {
      await archivePost({ variables: { postId } });
    }
  };

  const handleDelete = async (needId: string) => {
    if (window.confirm('Are you sure you want to delete this need? This action cannot be undone.')) {
      await deleteNeed({ variables: { needId } });
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
        <div className="text-stone-600">Loading needs...</div>
      </div>
    );
  }

  // Filter needs by organization name
  const filteredNeeds = needsData?.needs.nodes.filter(
    (need) => need.organizationName === organizationName
  ) || [];

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-6xl mx-auto">
        <div className="flex items-center mb-6">
          <button
            onClick={() => navigate('/resources')}
            className="text-amber-700 hover:text-amber-900 mr-4"
          >
            ← Back to Resources
          </button>
          <h1 className="text-3xl font-bold text-stone-900">{organizationName}</h1>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-800 rounded text-sm">
            {error}
          </div>
        )}

        <div className="space-y-4">
          {filteredNeeds.map((need) => (
            <NeedCard
              key={need.id}
              need={need}
              onApprove={handleApprove}
              onReject={handleReject}
              onUnpublish={handleUnpublish}
              onDelete={handleDelete}
              getStatusColor={getStatusColor}
            />
          ))}

          {filteredNeeds.length === 0 && (
            <div className="bg-white rounded-lg shadow-md p-12 text-center text-stone-600">
              No needs found for this organization. Run the scraper to fetch data.
            </div>
          )}
        </div>

        {/* Reject Modal */}
        {showRejectModal && (
          <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
            <div className="bg-white rounded-lg shadow-xl p-6 max-w-md w-full">
              <h2 className="text-xl font-semibold text-stone-900 mb-4">Reject Need</h2>
              <div className="mb-4">
                <label className="block text-sm font-medium text-stone-700 mb-2">
                  Rejection Reason
                </label>
                <textarea
                  value={rejectReason}
                  onChange={(e) => setRejectReason(e.target.value)}
                  placeholder="Explain why this need is being rejected..."
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

function NeedCard({
  need,
  onApprove,
  onReject,
  onUnpublish,
  onDelete,
  getStatusColor,
}: {
  need: Need;
  onApprove: (id: string) => void;
  onReject: (id: string) => void;
  onUnpublish: (postId: string) => void;
  onDelete: (id: string) => void;
  getStatusColor: (status: string) => string;
}) {
  const { data: postsData } = useQuery<{ postsForNeed: Post[] }>(GET_POSTS_FOR_NEED, {
    variables: { needId: need.id },
  });

  const activePosts = postsData?.postsForNeed.filter(
    (post) => post.status === 'published' || post.status === 'active'
  ) || [];

  const isPending = need.status.toLowerCase() === 'pending_approval';
  const isActive = need.status.toLowerCase() === 'active';

  return (
    <div className="bg-white rounded-lg shadow-md p-6">
      <div className="flex justify-between items-start mb-4">
        <div className="flex-1">
          <h3 className="text-lg font-semibold text-stone-900 mb-2">{need.title}</h3>
          <p className="text-sm text-stone-600 mb-2">{need.tldr}</p>
          {need.sourceUrl && (
            <a
              href={need.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-amber-700 hover:text-amber-900 hover:underline inline-flex items-center gap-1 mb-2"
            >
              View source page ↗
            </a>
          )}
          <div>
            <span className={`inline-block px-2 py-1 text-xs rounded-full ${getStatusColor(need.status)}`}>
              {need.status}
            </span>
          </div>
        </div>
      </div>

      {need.description && (
        <div className="mb-4 p-3 bg-stone-50 rounded text-sm text-stone-700">
          {need.description}
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
                onClick={() => onApprove(need.id)}
                className="bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700 text-sm"
              >
                ✓ Approve
              </button>
              <button
                onClick={() => onReject(need.id)}
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
            onClick={() => onDelete(need.id)}
            className="bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700 text-sm"
          >
            Delete
          </button>
        </div>
      </div>
    </div>
  );
}
