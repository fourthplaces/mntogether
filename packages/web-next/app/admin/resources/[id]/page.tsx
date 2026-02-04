"use client";

import Link from "next/link";
import { useState } from "react";
import { useParams } from "next/navigation";
import { useGraphQL, graphqlMutateClient, invalidateAllMatchingQuery } from "@/lib/graphql/client";
import { GET_RESOURCE } from "@/lib/graphql/queries";
import { APPROVE_RESOURCE, REJECT_RESOURCE } from "@/lib/graphql/mutations";
import type { Resource, GetResourceResult } from "@/lib/types";

export default function ResourceDetailPage() {
  const params = useParams();
  const resourceId = params.id as string;
  const [actionInProgress, setActionInProgress] = useState<string | null>(null);

  const {
    data,
    isLoading,
    error,
    mutate: refetch,
  } = useGraphQL<GetResourceResult>(GET_RESOURCE, { id: resourceId }, { revalidateOnFocus: false });

  const resource = data?.resource;

  const handleApprove = async () => {
    if (!confirm("Approve this resource?")) return;

    setActionInProgress("approve");
    try {
      await graphqlMutateClient(APPROVE_RESOURCE, { resourceId });
      refetch();
    } catch (err) {
      console.error("Failed to approve:", err);
      alert("Failed to approve resource");
    } finally {
      setActionInProgress(null);
    }
  };

  const handleReject = async () => {
    const reason = prompt("Reason for rejection:");
    if (reason === null) return;

    setActionInProgress("reject");
    try {
      await graphqlMutateClient(REJECT_RESOURCE, { resourceId, reason: reason || "Rejected" });
      refetch();
    } catch (err) {
      console.error("Failed to reject:", err);
      alert("Failed to reject resource");
    } finally {
      setActionInProgress(null);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status?.toUpperCase()) {
      case "APPROVED":
      case "ACTIVE":
        return "bg-green-100 text-green-800";
      case "PENDING":
      case "PENDING_APPROVAL":
        return "bg-yellow-100 text-yellow-800";
      case "REJECTED":
        return "bg-red-100 text-red-800";
      default:
        return "bg-gray-100 text-gray-800";
    }
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading resource...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Resource</h1>
            <p className="text-stone-600 mb-4">{error.message}</p>
            <Link href="/admin/resources" className="text-blue-600 hover:text-blue-800">
              Back to Resources
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!resource) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Resource Not Found</h1>
            <Link href="/admin/resources" className="text-blue-600 hover:text-blue-800">
              Back to Resources
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
        <Link
          href="/admin/resources"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Resources
        </Link>

        {/* Resource Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start mb-4">
            <div className="flex-1">
              <h1 className="text-2xl font-bold text-stone-900 mb-2">{resource.title}</h1>
              {resource.organizationName && (
                <p className="text-lg text-stone-600">{resource.organizationName}</p>
              )}
            </div>
            <div className="flex items-center gap-2">
              <span className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusColor(resource.status)}`}>
                {resource.status}
              </span>
            </div>
          </div>

          {/* Action Buttons */}
          {resource.status === "PENDING" && (
            <div className="flex gap-2 mb-4">
              <button
                onClick={handleApprove}
                disabled={actionInProgress !== null}
                className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
              >
                {actionInProgress === "approve" ? "..." : "Approve"}
              </button>
              <button
                onClick={handleReject}
                disabled={actionInProgress !== null}
                className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
              >
                {actionInProgress === "reject" ? "..." : "Reject"}
              </button>
            </div>
          )}

          {/* Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-3 gap-4 pt-4 border-t border-stone-200">
            {resource.location && (
              <div>
                <span className="text-xs text-stone-500 uppercase">Location</span>
                <p className="text-sm font-medium text-stone-900">{resource.location}</p>
              </div>
            )}
            {resource.resourceType && (
              <div>
                <span className="text-xs text-stone-500 uppercase">Type</span>
                <p className="text-sm font-medium text-stone-900">{resource.resourceType}</p>
              </div>
            )}
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(resource.createdAt)}</p>
            </div>
            {resource.sourceUrl && (
              <div className="col-span-2">
                <span className="text-xs text-stone-500 uppercase">Source URL</span>
                <p className="text-sm font-medium">
                  <a
                    href={resource.sourceUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:underline"
                  >
                    {resource.sourceUrl}
                  </a>
                </p>
              </div>
            )}
          </div>
        </div>

        {/* Content */}
        <div className="bg-white rounded-lg shadow-md p-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">Content</h2>
          <div className="prose prose-stone max-w-none">
            <p className="text-stone-700 whitespace-pre-wrap">{resource.content}</p>
          </div>
        </div>
      </div>
    </div>
  );
}
