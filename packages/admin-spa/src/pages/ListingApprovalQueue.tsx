import { useQuery, useMutation } from '@apollo/client';
import { GET_PENDING_LISTINGS } from '../graphql/queries';
import { APPROVE_LISTING, REJECT_LISTING } from '../graphql/mutations';
import { useState } from 'react';

interface Listing {
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

export function ListingApprovalQueue() {
  const [selectedListing, setSelectedListing] = useState<Listing | null>(null);
  const { data, loading, error, refetch } = useQuery(GET_PENDING_LISTINGS, {
    variables: { limit: 50, offset: 0 },
  });

  const [approveListing] = useMutation(APPROVE_LISTING, {
    onCompleted: () => {
      refetch();
      setSelectedListing(null);
    },
  });

  const [rejectListing] = useMutation(REJECT_LISTING, {
    onCompleted: () => {
      refetch();
      setSelectedListing(null);
    },
  });

  if (loading) return <div className="p-8">Loading...</div>;
  if (error) return <div className="p-8 text-red-600">Error: {error.message}</div>;

  const listings = data?.listings?.nodes || [];

  const handleApprove = async (listingId: string) => {
    if (confirm('Approve this listing? It will become visible to all volunteers.')) {
      await approveListing({ variables: { listingId } });
    }
  };

  const handleReject = async (listingId: string) => {
    const reason = prompt('Reason for rejection (optional):');
    if (reason !== null) {
      await rejectListing({ variables: { listingId, reason: reason || 'Rejected by admin' } });
    }
  };

  return (
    <div className="max-w-7xl mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">Listing Approval Queue</h1>

      {listings.length === 0 ? (
        <div className="text-stone-500 text-center py-12">
          No pending listings to review
        </div>
      ) : (
        <div className="grid gap-6">
          {listings.map((listing: Listing) => (
            <div
              key={listing.id}
              className="bg-white border border-stone-200 rounded-lg p-6 hover:shadow-lg transition-shadow"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    <span className="text-xs font-medium px-2 py-1 bg-stone-100 rounded">
                      {listing.submissionType === 'user_submitted' ? '👤 User' : '🌐 Scraped'}
                    </span>
                    {listing.urgency && (
                      <span className={`text-xs font-medium px-2 py-1 rounded ${
                        listing.urgency === 'urgent' ? 'bg-red-100 text-red-700' :
                        listing.urgency === 'low' ? 'bg-amber-100 text-amber-700' :
                        'bg-yellow-100 text-yellow-700'
                      }`}>
                        {listing.urgency}
                      </span>
                    )}
                  </div>
                  <h3 className="text-xl font-semibold mb-1">{listing.title}</h3>
                  <p className="text-sm text-stone-600 mb-2">{listing.organizationName}</p>
                  {listing.location && (
                    <p className="text-sm text-stone-500 mb-2">📍 {listing.location}</p>
                  )}
                  <p className="text-stone-700 mb-4">{listing.tldr}</p>
                </div>
              </div>

              <div className="flex gap-2">
                <button
                  onClick={() => setSelectedListing(listing)}
                  className="px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700"
                >
                  View Details
                </button>
                <button
                  onClick={() => handleApprove(listing.id)}
                  className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
                >
                  ✓ Approve
                </button>
                <button
                  onClick={() => handleReject(listing.id)}
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
      {selectedListing && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4">
          <div className="bg-white rounded-lg max-w-2xl w-full max-h-[80vh] overflow-y-auto p-6">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-2xl font-bold">{selectedListing.title}</h2>
              <button
                onClick={() => setSelectedListing(null)}
                className="text-stone-500 hover:text-stone-700"
              >
                ✕
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <h3 className="font-semibold text-stone-700">Organization</h3>
                <p>{selectedListing.organizationName}</p>
              </div>

              {selectedListing.location && (
                <div>
                  <h3 className="font-semibold text-stone-700">Location</h3>
                  <p>{selectedListing.location}</p>
                </div>
              )}

              <div>
                <h3 className="font-semibold text-stone-700">Description</h3>
                <p className="whitespace-pre-wrap">{selectedListing.description}</p>
              </div>

              {selectedListing.contactInfo && (
                <div>
                  <h3 className="font-semibold text-stone-700">Contact</h3>
                  {selectedListing.contactInfo.email && <p>Email: {selectedListing.contactInfo.email}</p>}
                  {selectedListing.contactInfo.phone && <p>Phone: {selectedListing.contactInfo.phone}</p>}
                  {selectedListing.contactInfo.website && (
                    <p>
                      Website:{' '}
                      <a
                        href={selectedListing.contactInfo.website}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-amber-600 hover:underline"
                      >
                        {selectedListing.contactInfo.website}
                      </a>
                    </p>
                  )}
                </div>
              )}
            </div>

            <div className="flex gap-2 mt-6">
              <button
                onClick={() => handleApprove(selectedListing.id)}
                className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700"
              >
                ✓ Approve
              </button>
              <button
                onClick={() => handleReject(selectedListing.id)}
                className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
              >
                ✗ Reject
              </button>
              <button
                onClick={() => setSelectedListing(null)}
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
