import { useQuery, useMutation } from '@apollo/client';
import { GET_PENDING_NEEDS } from '../graphql/queries';
import { APPROVE_NEED, REJECT_NEED } from '../graphql/mutations';
import { useState } from 'react';

interface Need {
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

export function NeedApprovalQueue() {
  const [selectedNeed, setSelectedNeed] = useState<Need | null>(null);
  const { data, loading, error, refetch } = useQuery(GET_PENDING_NEEDS, {
    variables: { limit: 50, offset: 0 },
  });

  const [approveNeed] = useMutation(APPROVE_NEED, {
    onCompleted: () => {
      refetch();
      setSelectedNeed(null);
    },
  });

  const [rejectNeed] = useMutation(REJECT_NEED, {
    onCompleted: () => {
      refetch();
      setSelectedNeed(null);
    },
  });

  if (loading) return <div className="p-8">Loading...</div>;
  if (error) return <div className="p-8 text-red-600">Error: {error.message}</div>;

  const needs = data?.needs?.nodes || [];

  const handleApprove = async (needId: string) => {
    if (confirm('Approve this need? It will become visible to all volunteers.')) {
      await approveNeed({ variables: { needId } });
    }
  };

  const handleReject = async (needId: string) => {
    const reason = prompt('Reason for rejection (optional):');
    if (reason !== null) {
      await rejectNeed({ variables: { needId, reason: reason || 'Rejected by admin' } });
    }
  };

  return (
    <div className="max-w-7xl mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">Need Approval Queue</h1>

      {needs.length === 0 ? (
        <div className="text-stone-500 text-center py-12">
          No pending needs to review
        </div>
      ) : (
        <div className="grid gap-6">
          {needs.map((need: Need) => (
            <div
              key={need.id}
              className="bg-white border border-stone-200 rounded-lg p-6 hover:shadow-lg transition-shadow"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-xs font-medium px-2 py-1 bg-stone-100 rounded">
                      {need.submissionType === 'user_submitted' ? '👤 User' : '🌐 Scraped'}
                    </span>
                    {need.urgency && (
                      <span className={`text-xs font-medium px-2 py-1 rounded ${
                        need.urgency === 'urgent' ? 'bg-red-100 text-red-700' :
                        need.urgency === 'low' ? 'bg-amber-100 text-amber-700' :
                        'bg-yellow-100 text-yellow-700'
                      }`}>
                        {need.urgency}
                      </span>
                    )}
                  </div>
                  <h3 className="text-xl font-semibold mb-1">{need.title}</h3>
                  <p className="text-sm text-stone-600 mb-2">{need.organizationName}</p>
                  {need.location && (
                    <p className="text-sm text-stone-500 mb-2">📍 {need.location}</p>
                  )}
                  <p className="text-stone-700 mb-4">{need.tldr}</p>
                </div>
              </div>

              <div className="flex gap-2">
                <button
                  onClick={() => setSelectedNeed(need)}
                  className="px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700"
                >
                  View Details
                </button>
                <button
                  onClick={() => handleApprove(need.id)}
                  className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
                >
                  ✓ Approve
                </button>
                <button
                  onClick={() => handleReject(need.id)}
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
      {selectedNeed && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4">
          <div className="bg-white rounded-lg max-w-2xl w-full max-h-[80vh] overflow-y-auto p-6">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-2xl font-bold">{selectedNeed.title}</h2>
              <button
                onClick={() => setSelectedNeed(null)}
                className="text-stone-500 hover:text-stone-700"
              >
                ✕
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <h3 className="font-semibold text-stone-700">Organization</h3>
                <p>{selectedNeed.organizationName}</p>
              </div>

              {selectedNeed.location && (
                <div>
                  <h3 className="font-semibold text-stone-700">Location</h3>
                  <p>{selectedNeed.location}</p>
                </div>
              )}

              <div>
                <h3 className="font-semibold text-stone-700">Description</h3>
                <p className="whitespace-pre-wrap">{selectedNeed.description}</p>
              </div>

              {selectedNeed.contactInfo && (
                <div>
                  <h3 className="font-semibold text-stone-700">Contact</h3>
                  {selectedNeed.contactInfo.email && <p>Email: {selectedNeed.contactInfo.email}</p>}
                  {selectedNeed.contactInfo.phone && <p>Phone: {selectedNeed.contactInfo.phone}</p>}
                  {selectedNeed.contactInfo.website && (
                    <p>
                      Website:{' '}
                      <a
                        href={selectedNeed.contactInfo.website}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-amber-600 hover:underline"
                      >
                        {selectedNeed.contactInfo.website}
                      </a>
                    </p>
                  )}
                </div>
              )}
            </div>

            <div className="flex gap-2 mt-6">
              <button
                onClick={() => handleApprove(selectedNeed.id)}
                className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
              >
                ✓ Approve
              </button>
              <button
                onClick={() => handleReject(selectedNeed.id)}
                className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
              >
                ✗ Reject
              </button>
              <button
                onClick={() => setSelectedNeed(null)}
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
