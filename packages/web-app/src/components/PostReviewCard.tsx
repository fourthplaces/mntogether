import React, { useState } from 'react';

interface ContactInfo {
  email?: string;
  phone?: string;
  website?: string;
}

interface ServiceFields {
  requiresIdentification?: boolean;
  requiresAppointment?: boolean;
  walkInsAccepted?: boolean;
  remoteAvailable?: boolean;
  inPersonAvailable?: boolean;
  homeVisitsAvailable?: boolean;
  wheelchairAccessible?: boolean;
  interpretationAvailable?: boolean;
  freeService?: boolean;
  slidingScaleFees?: boolean;
  acceptsInsurance?: boolean;
  eveningHours?: boolean;
  weekendHours?: boolean;
}

interface OpportunityFields {
  opportunityType?: string;
  timeCommitment?: string;
  requiresBackgroundCheck?: boolean;
  minimumAge?: number;
  skillsNeeded?: string[];
  remoteOk?: boolean;
}

interface BusinessFields {
  businessInfo?: {
    proceedsPercentage?: number;
    proceedsBeneficiary?: {
      id: string;
      name: string;
    };
    donationLink?: string;
    giftCardLink?: string;
    onlineStoreUrl?: string;
  };
}

interface Post {
  id: string;
  postType: 'service' | 'opportunity' | 'business';
  organizationName: string;
  title: string;
  tldr?: string;
  description: string;
  contactInfo?: ContactInfo;
  urgency?: string;
  location?: string;
  category?: string;
  sourceUrl?: string;
  createdAt: string;
}

type PostWithTypeFields = Post & (ServiceFields | OpportunityFields | BusinessFields);

interface PostReviewCardProps {
  listing: PostWithTypeFields;
  onApprove: (id: string) => void;
  onReject: (id: string, reason?: string) => void;
  onEdit: (post: PostWithTypeFields) => void;
}

const PostReviewCard: React.FC<PostReviewCardProps> = ({
  listing,
  onApprove,
  onReject,
  onEdit,
}) => {
  const [expanded, setExpanded] = useState(false);
  const [showRejectModal, setShowRejectModal] = useState(false);
  const [rejectReason, setRejectReason] = useState('');

  const post = listing; // Alias for readability

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'service':
        return 'bg-blue-100 text-blue-800';
      case 'opportunity':
        return 'bg-green-100 text-green-800';
      case 'business':
        return 'bg-purple-100 text-purple-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const getUrgencyColor = (urgency?: string) => {
    switch (urgency?.toLowerCase()) {
      case 'urgent':
        return 'bg-red-100 text-red-800';
      case 'high':
        return 'bg-orange-100 text-orange-800';
      case 'medium':
        return 'bg-yellow-100 text-yellow-800';
      case 'low':
        return 'bg-green-100 text-green-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  const handleReject = () => {
    onReject(post.id, rejectReason);
    setShowRejectModal(false);
    setRejectReason('');
  };

  const renderTypeSpecificFields = () => {
    if (post.postType === 'service') {
      const service = post as Post & ServiceFields;
      const features = [];

      if (service.freeService) features.push('Free');
      if (service.slidingScaleFees) features.push('Sliding Scale');
      if (service.acceptsInsurance) features.push('Accepts Insurance');
      if (service.remoteAvailable) features.push('Remote');
      if (service.inPersonAvailable) features.push('In-Person');
      if (service.homeVisitsAvailable) features.push('Home Visits');
      if (service.walkInsAccepted) features.push('Walk-Ins OK');
      if (service.wheelchairAccessible) features.push('Wheelchair Accessible');
      if (service.interpretationAvailable) features.push('Interpretation');
      if (service.eveningHours) features.push('Evening Hours');
      if (service.weekendHours) features.push('Weekend Hours');
      if (!service.requiresIdentification) features.push('No ID Required');

      return (
        <div className="mt-3 space-y-2">
          <h4 className="font-semibold text-sm text-stone-700">Service Features:</h4>
          <div className="flex flex-wrap gap-1">
            {features.map((feature, idx) => (
              <span
                key={idx}
                className="px-2 py-1 text-xs bg-blue-50 text-blue-700 rounded"
              >
                {feature}
              </span>
            ))}
          </div>
        </div>
      );
    }

    if (post.postType === 'opportunity') {
      const opportunity = post as Post & OpportunityFields;
      return (
        <div className="mt-3 space-y-2">
          <h4 className="font-semibold text-sm text-stone-700">Opportunity Details:</h4>
          <div className="grid grid-cols-2 gap-2 text-sm">
            {opportunity.opportunityType && (
              <div>
                <span className="font-medium">Type:</span> {opportunity.opportunityType}
              </div>
            )}
            {opportunity.timeCommitment && (
              <div>
                <span className="font-medium">Time:</span> {opportunity.timeCommitment}
              </div>
            )}
            {opportunity.minimumAge && (
              <div>
                <span className="font-medium">Min Age:</span> {opportunity.minimumAge}
              </div>
            )}
            {opportunity.remoteOk !== undefined && (
              <div>
                <span className="font-medium">Remote:</span> {opportunity.remoteOk ? 'Yes' : 'No'}
              </div>
            )}
            {opportunity.requiresBackgroundCheck !== undefined && (
              <div>
                <span className="font-medium">Background Check:</span>{' '}
                {opportunity.requiresBackgroundCheck ? 'Required' : 'Not Required'}
              </div>
            )}
          </div>
          {opportunity.skillsNeeded && opportunity.skillsNeeded.length > 0 && (
            <div>
              <span className="font-medium text-sm">Skills Needed:</span>
              <div className="flex flex-wrap gap-1 mt-1">
                {opportunity.skillsNeeded.map((skill, idx) => (
                  <span key={idx} className="px-2 py-1 text-xs bg-green-50 text-green-700 rounded">
                    {skill}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      );
    }

    if (post.postType === 'business') {
      const business = post as Post & BusinessFields;
      return (
        <div className="mt-3 space-y-2">
          <h4 className="font-semibold text-sm text-stone-700">Business Details:</h4>
          {business.businessInfo && (
            <div className="space-y-2 text-sm">
              {business.businessInfo.proceedsPercentage && (
                <div>
                  <span className="font-medium">Proceeds:</span>{' '}
                  {business.businessInfo.proceedsPercentage}% donated
                  {business.businessInfo.proceedsBeneficiary && (
                    <span> to {business.businessInfo.proceedsBeneficiary.name}</span>
                  )}
                </div>
              )}
              <div className="flex flex-wrap gap-2">
                {business.businessInfo.onlineStoreUrl && (
                  <a
                    href={business.businessInfo.onlineStoreUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs px-2 py-1 bg-purple-50 text-purple-700 rounded hover:bg-purple-100"
                  >
                    Store
                  </a>
                )}
                {business.businessInfo.donationLink && (
                  <a
                    href={business.businessInfo.donationLink}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs px-2 py-1 bg-purple-50 text-purple-700 rounded hover:bg-purple-100"
                  >
                    Donate
                  </a>
                )}
                {business.businessInfo.giftCardLink && (
                  <a
                    href={business.businessInfo.giftCardLink}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs px-2 py-1 bg-purple-50 text-purple-700 rounded hover:bg-purple-100"
                  >
                    Gift Card
                  </a>
                )}
              </div>
            </div>
          )}
        </div>
      );
    }

    return null;
  };

  return (
    <>
      <div className="bg-white border border-stone-200 rounded-lg shadow-sm p-4 hover:shadow-md transition-shadow">
        {/* Header */}
        <div className="flex items-start justify-between mb-2">
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className={`px-2 py-1 text-xs font-medium rounded ${getTypeColor(post.postType)}`}>
                {post.postType}
              </span>
              {post.urgency && (
                <span className={`px-2 py-1 text-xs font-medium rounded ${getUrgencyColor(post.urgency)}`}>
                  {post.urgency}
                </span>
              )}
              {post.category && (
                <span className="px-2 py-1 text-xs bg-stone-100 text-stone-700 rounded">
                  {post.category}
                </span>
              )}
            </div>
            <h3 className="text-lg font-semibold text-stone-900">{post.title}</h3>
            <p className="text-sm text-stone-600">{post.organizationName}</p>
          </div>
        </div>

        {/* TLDR */}
        {post.tldr && (
          <p className="text-sm text-stone-700 italic mb-2">"{post.tldr}"</p>
        )}

        {/* Description (collapsed) */}
        <p className={`text-sm text-stone-600 ${!expanded && 'line-clamp-2'}`}>
          {post.description}
        </p>

        {/* Expand button */}
        <button
          onClick={() => setExpanded(!expanded)}
          className="text-xs text-amber-600 hover:text-amber-800 mt-1"
        >
          {expanded ? 'Show less' : 'Show more'}
        </button>

        {/* Expanded details */}
        {expanded && (
          <div className="mt-3 space-y-3 pt-3 border-t border-stone-200">
            {/* Contact Info */}
            {post.contactInfo && (
              <div>
                <h4 className="font-semibold text-sm text-stone-700 mb-1">Contact:</h4>
                <div className="text-sm text-stone-600 space-y-1">
                  {post.contactInfo.email && (
                    <div>Email: {post.contactInfo.email}</div>
                  )}
                  {post.contactInfo.phone && (
                    <div>Phone: {post.contactInfo.phone}</div>
                  )}
                  {post.contactInfo.website && (
                    <div>
                      Website:{' '}
                      <a
                        href={post.contactInfo.website}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-amber-600 hover:text-amber-800"
                      >
                        {post.contactInfo.website}
                      </a>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Location */}
            {post.location && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Location:</span>{' '}
                <span className="text-sm text-stone-600">{post.location}</span>
              </div>
            )}

            {/* Source URL */}
            {post.sourceUrl && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Source:</span>{' '}
                <a
                  href={post.sourceUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-amber-600 hover:text-amber-800 break-all"
                >
                  {post.sourceUrl}
                </a>
              </div>
            )}

            {/* Type-specific fields */}
            {renderTypeSpecificFields()}
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-2 mt-4 pt-3 border-t border-stone-200">
          <button
            onClick={() => onApprove(post.id)}
            className="flex-1 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors font-medium"
          >
            Approve
          </button>
          <button
            onClick={() => onEdit(listing)}
            className="flex-1 px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700 transition-colors font-medium"
          >
            Edit
          </button>
          <button
            onClick={() => setShowRejectModal(true)}
            className="flex-1 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 transition-colors font-medium"
          >
            Reject
          </button>
        </div>
      </div>

      {/* Reject Modal */}
      {showRejectModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg p-6 max-w-md w-full">
            <h3 className="text-lg font-semibold text-stone-900 mb-4">
              Reject Post
            </h3>
            <p className="text-sm text-stone-600 mb-4">
              Are you sure you want to reject "{post.title}"? You can optionally provide a reason.
            </p>
            <textarea
              value={rejectReason}
              onChange={(e) => setRejectReason(e.target.value)}
              placeholder="Reason for rejection (optional)"
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500 mb-4"
              rows={3}
            />
            <div className="flex gap-2">
              <button
                onClick={handleReject}
                className="flex-1 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700"
              >
                Reject
              </button>
              <button
                onClick={() => {
                  setShowRejectModal(false);
                  setRejectReason('');
                }}
                className="flex-1 px-4 py-2 bg-stone-200 text-stone-700 rounded hover:bg-stone-300"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};

export default PostReviewCard;
