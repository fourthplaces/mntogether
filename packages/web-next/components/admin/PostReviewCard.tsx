"use client";

import { useState } from "react";
import type { Post } from "@/lib/types";

interface PostReviewCardProps {
  post: Post;
  onApprove: (id: string) => void;
  onReject: (id: string, reason?: string) => void;
  onEdit: (post: Post) => void;
  isApproving?: boolean;
  isRejecting?: boolean;
}

export function PostReviewCard({
  post,
  onApprove,
  onReject,
  onEdit,
  isApproving,
  isRejecting,
}: PostReviewCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [showRejectModal, setShowRejectModal] = useState(false);
  const [rejectReason, setRejectReason] = useState("");

  const getTypeColor = (type?: string) => {
    switch (type) {
      case "service":
        return "bg-blue-100 text-blue-800";
      case "opportunity":
        return "bg-green-100 text-green-800";
      case "business":
        return "bg-purple-100 text-purple-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const getUrgencyColor = (urgency?: string) => {
    switch (urgency?.toLowerCase()) {
      case "urgent":
        return "bg-red-100 text-red-800";
      case "high":
        return "bg-orange-100 text-orange-800";
      case "medium":
        return "bg-yellow-100 text-yellow-800";
      case "low":
        return "bg-green-100 text-green-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const handleReject = () => {
    onReject(post.id, rejectReason);
    setShowRejectModal(false);
    setRejectReason("");
  };

  const renderServiceFeatures = () => {
    const features = [];
    if (post.freeService) features.push("Free");
    if (post.slidingScaleFees) features.push("Sliding Scale");
    if (post.acceptsInsurance) features.push("Accepts Insurance");
    if (post.remoteAvailable) features.push("Remote");
    if (post.inPersonAvailable) features.push("In-Person");
    if (post.homeVisitsAvailable) features.push("Home Visits");
    if (post.walkInsAccepted) features.push("Walk-Ins OK");
    if (post.wheelchairAccessible) features.push("Wheelchair Accessible");
    if (post.interpretationAvailable) features.push("Interpretation");
    if (post.eveningHours) features.push("Evening Hours");
    if (post.weekendHours) features.push("Weekend Hours");
    if (!post.requiresIdentification) features.push("No ID Required");

    if (features.length === 0) return null;

    return (
      <div className="mt-3 space-y-2">
        <h4 className="font-semibold text-sm text-stone-700">Service Features:</h4>
        <div className="flex flex-wrap gap-1">
          {features.map((feature, idx) => (
            <span key={idx} className="px-2 py-1 text-xs bg-blue-50 text-blue-700 rounded">
              {feature}
            </span>
          ))}
        </div>
      </div>
    );
  };

  const renderOpportunityDetails = () => {
    if (post.postType !== "opportunity") return null;

    return (
      <div className="mt-3 space-y-2">
        <h4 className="font-semibold text-sm text-stone-700">Opportunity Details:</h4>
        <div className="grid grid-cols-2 gap-2 text-sm">
          {post.opportunityType && (
            <div>
              <span className="font-medium">Type:</span> {post.opportunityType}
            </div>
          )}
          {post.timeCommitment && (
            <div>
              <span className="font-medium">Time:</span> {post.timeCommitment}
            </div>
          )}
          {post.minimumAge && (
            <div>
              <span className="font-medium">Min Age:</span> {post.minimumAge}
            </div>
          )}
          {post.remoteOk !== undefined && (
            <div>
              <span className="font-medium">Remote:</span> {post.remoteOk ? "Yes" : "No"}
            </div>
          )}
          {post.requiresBackgroundCheck !== undefined && (
            <div>
              <span className="font-medium">Background Check:</span>{" "}
              {post.requiresBackgroundCheck ? "Required" : "Not Required"}
            </div>
          )}
        </div>
        {post.skillsNeeded && post.skillsNeeded.length > 0 && (
          <div>
            <span className="font-medium text-sm">Skills Needed:</span>
            <div className="flex flex-wrap gap-1 mt-1">
              {post.skillsNeeded.map((skill, idx) => (
                <span key={idx} className="px-2 py-1 text-xs bg-green-50 text-green-700 rounded">
                  {skill}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    );
  };

  const renderBusinessDetails = () => {
    if (post.postType !== "business" || !post.businessInfo) return null;

    return (
      <div className="mt-3 space-y-2">
        <h4 className="font-semibold text-sm text-stone-700">Business Details:</h4>
        <div className="space-y-2 text-sm">
          {post.businessInfo.proceedsPercentage && (
            <div>
              <span className="font-medium">Proceeds:</span> {post.businessInfo.proceedsPercentage}%
              donated
              {post.businessInfo.proceedsBeneficiary && (
                <span> to {post.businessInfo.proceedsBeneficiary.name}</span>
              )}
            </div>
          )}
          <div className="flex flex-wrap gap-2">
            {post.businessInfo.onlineStoreUrl && (
              <a
                href={post.businessInfo.onlineStoreUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs px-2 py-1 bg-purple-50 text-purple-700 rounded hover:bg-purple-100"
              >
                Store
              </a>
            )}
            {post.businessInfo.donationLink && (
              <a
                href={post.businessInfo.donationLink}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs px-2 py-1 bg-purple-50 text-purple-700 rounded hover:bg-purple-100"
              >
                Donate
              </a>
            )}
            {post.businessInfo.giftCardLink && (
              <a
                href={post.businessInfo.giftCardLink}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs px-2 py-1 bg-purple-50 text-purple-700 rounded hover:bg-purple-100"
              >
                Gift Card
              </a>
            )}
          </div>
        </div>
      </div>
    );
  };

  return (
    <>
      <div className="bg-white border border-stone-200 rounded-lg shadow-sm p-4 hover:shadow-md transition-shadow">
        {/* Header */}
        <div className="flex items-start justify-between mb-2">
          <div className="flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className={`px-2 py-1 text-xs font-medium rounded ${getTypeColor(post.postType)}`}>
                {post.postType || "post"}
              </span>
              {post.urgency && (
                <span
                  className={`px-2 py-1 text-xs font-medium rounded ${getUrgencyColor(post.urgency)}`}
                >
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
        {post.tldr && <p className="text-sm text-stone-700 italic mb-2">&quot;{post.tldr}&quot;</p>}

        {/* Description (collapsed) */}
        <p className={`text-sm text-stone-600 ${!expanded && "line-clamp-2"}`}>{post.description}</p>

        {/* Expand button */}
        <button
          onClick={() => setExpanded(!expanded)}
          className="text-xs text-amber-600 hover:text-amber-800 mt-1"
        >
          {expanded ? "Show less" : "Show more"}
        </button>

        {/* Expanded details */}
        {expanded && (
          <div className="mt-3 space-y-3 pt-3 border-t border-stone-200">
            {/* Location */}
            {post.location && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Location:</span>{" "}
                <span className="text-sm text-stone-600">{post.location}</span>
              </div>
            )}

            {/* Source URL */}
            {post.sourceUrl && (
              <div>
                <span className="font-semibold text-sm text-stone-700">Source:</span>{" "}
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
            {post.postType === "service" && renderServiceFeatures()}
            {post.postType === "opportunity" && renderOpportunityDetails()}
            {post.postType === "business" && renderBusinessDetails()}
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-2 mt-4 pt-3 border-t border-stone-200">
          <button
            onClick={() => onApprove(post.id)}
            disabled={isApproving}
            className="flex-1 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 transition-colors font-medium disabled:opacity-50"
          >
            {isApproving ? "..." : "Approve"}
          </button>
          <button
            onClick={() => onEdit(post)}
            className="flex-1 px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700 transition-colors font-medium"
          >
            Edit
          </button>
          <button
            onClick={() => setShowRejectModal(true)}
            disabled={isRejecting}
            className="flex-1 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 transition-colors font-medium disabled:opacity-50"
          >
            {isRejecting ? "..." : "Reject"}
          </button>
        </div>
      </div>

      {/* Reject Modal */}
      {showRejectModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg p-6 max-w-md w-full">
            <h3 className="text-lg font-semibold text-stone-900 mb-4">Reject Post</h3>
            <p className="text-sm text-stone-600 mb-4">
              Are you sure you want to reject &quot;{post.title}&quot;? You can optionally provide a
              reason.
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
                  setRejectReason("");
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
}
