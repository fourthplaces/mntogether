"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useGraphQL } from "@/lib/graphql/client";
import { GET_ORGANIZATION } from "@/lib/graphql/queries";

interface Organization {
  id: string;
  name: string;
  description: string | null;
  location: string | null;
  contactInfo: {
    email?: string;
    phone?: string;
    website?: string;
  } | null;
  createdAt: string;
  updatedAt: string;
  posts?: Array<{
    id: string;
    title: string;
    status: string;
    createdAt: string;
  }>;
}

interface GetOrganizationResult {
  organization: Organization | null;
}

export default function OrganizationDetailPage() {
  const params = useParams();
  const organizationId = params.id as string;

  const { data, isLoading, error } = useGraphQL<GetOrganizationResult>(
    GET_ORGANIZATION,
    { id: organizationId },
    { revalidateOnFocus: false }
  );

  const organization = data?.organization;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading organization...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Organization</h1>
            <p className="text-stone-600 mb-4">{error.message}</p>
            <Link href="/admin/organizations" className="text-blue-600 hover:text-blue-800">
              Back to Organizations
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!organization) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Organization Not Found</h1>
            <Link href="/admin/organizations" className="text-blue-600 hover:text-blue-800">
              Back to Organizations
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
          href="/admin/organizations"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          {"\u2190"} Back to Organizations
        </Link>

        {/* Organization Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h1 className="text-2xl font-bold text-stone-900 mb-2">{organization.name}</h1>

          {organization.location && (
            <p className="text-stone-600 mb-4">
              {"\u{1F4CD}"} {organization.location}
            </p>
          )}

          {organization.description && (
            <p className="text-stone-700 mb-4">{organization.description}</p>
          )}

          {/* Contact Info */}
          {organization.contactInfo && (
            <div className="pt-4 border-t border-stone-200">
              <h3 className="text-sm font-semibold text-stone-900 mb-2">Contact Information</h3>
              <div className="space-y-1">
                {organization.contactInfo.email && (
                  <p className="text-sm text-stone-600">
                    <span className="font-medium">Email:</span>{" "}
                    <a href={`mailto:${organization.contactInfo.email}`} className="text-blue-600 hover:underline">
                      {organization.contactInfo.email}
                    </a>
                  </p>
                )}
                {organization.contactInfo.phone && (
                  <p className="text-sm text-stone-600">
                    <span className="font-medium">Phone:</span> {organization.contactInfo.phone}
                  </p>
                )}
                {organization.contactInfo.website && (
                  <p className="text-sm text-stone-600">
                    <span className="font-medium">Website:</span>{" "}
                    <a
                      href={organization.contactInfo.website}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:underline"
                    >
                      {organization.contactInfo.website}
                    </a>
                  </p>
                )}
              </div>
            </div>
          )}

          {/* Dates */}
          <div className="grid grid-cols-2 gap-4 pt-4 mt-4 border-t border-stone-200">
            <div>
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(organization.createdAt)}</p>
            </div>
            <div>
              <span className="text-xs text-stone-500 uppercase">Updated</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(organization.updatedAt)}</p>
            </div>
          </div>
        </div>

        {/* Posts */}
        {organization.posts && organization.posts.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">
              Posts ({organization.posts.length})
            </h2>
            <div className="space-y-3">
              {organization.posts.map((post) => (
                <Link
                  key={post.id}
                  href={`/admin/posts/${post.id}`}
                  className="block border border-stone-200 rounded-lg p-4 hover:bg-stone-50"
                >
                  <div className="flex justify-between items-start">
                    <h3 className="font-medium text-stone-900">{post.title}</h3>
                    <span
                      className={`text-xs px-2 py-1 rounded ${
                        post.status === "active"
                          ? "bg-green-100 text-green-800"
                          : post.status === "pending_approval"
                            ? "bg-amber-100 text-amber-800"
                            : "bg-stone-100 text-stone-800"
                      }`}
                    >
                      {post.status}
                    </span>
                  </div>
                  <p className="text-xs text-stone-500 mt-1">{formatDate(post.createdAt)}</p>
                </Link>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
