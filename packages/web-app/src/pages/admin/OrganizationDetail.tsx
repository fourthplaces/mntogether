import { useParams, useNavigate } from 'react-router-dom';
import { useQuery } from '@apollo/client';
import { GET_ORGANIZATION } from '../../graphql/queries';
import type { Organization } from '@/types/organization';

interface OrganizationResponse {
  organization: Organization;
}

export function OrganizationDetail() {
  const { sourceId } = useParams<{ sourceId: string }>();
  const navigate = useNavigate();

  const { data, loading, error } = useQuery<OrganizationResponse>(
    GET_ORGANIZATION,
    {
      variables: { id: sourceId },
      skip: !sourceId,
    }
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-stone-600">Loading...</div>
      </div>
    );
  }

  if (error || !data?.organization) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-stone-600">Organization not found</div>
      </div>
    );
  }

  const org = data.organization;

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-4xl mx-auto">
        <button
          onClick={() => navigate('/admin/organizations')}
          className="mb-6 text-stone-600 hover:text-stone-900 flex items-center gap-2"
        >
          <svg
            className="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M15 19l-7-7 7-7"
            />
          </svg>
          Back to Organizations
        </button>

        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex items-center gap-2 mb-2">
            <h1 className="text-2xl font-bold text-stone-900">
              {org.name}
            </h1>
            {org.verified && (
              <span className="inline-flex items-center gap-1 text-sm text-green-700">
                <span>✓</span>
                <span>Verified</span>
              </span>
            )}
          </div>
          {org.description && (
            <p className="text-stone-600 mb-4">{org.description}</p>
          )}
          {org.contactInfo?.website && (
            <a
              href={org.contactInfo.website}
              target="_blank"
              rel="noopener noreferrer"
              className="text-amber-700 hover:text-amber-900 text-sm"
            >
              {org.contactInfo.website} ↗
            </a>
          )}
        </div>

        {org.businessInfo?.isCauseDriven && (
          <div className="bg-green-50 rounded-lg shadow-md p-6 mb-6 border border-green-200">
            <h2 className="text-lg font-semibold text-green-900 mb-2">
              Cause-Driven Business
            </h2>
            {org.businessInfo.proceedsPercentage && (
              <p className="text-green-800 mb-2">
                Donates {org.businessInfo.proceedsPercentage}% of proceeds to charity
              </p>
            )}
            <div className="flex gap-3">
              {org.businessInfo.onlineStoreUrl && (
                <a
                  href={org.businessInfo.onlineStoreUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-green-700 hover:text-green-900 text-sm underline"
                >
                  Online Store ↗
                </a>
              )}
              {org.businessInfo.donationLink && (
                <a
                  href={org.businessInfo.donationLink}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-green-700 hover:text-green-900 text-sm underline"
                >
                  Donate ↗
                </a>
              )}
              {org.businessInfo.giftCardLink && (
                <a
                  href={org.businessInfo.giftCardLink}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-green-700 hover:text-green-900 text-sm underline"
                >
                  Gift Cards ↗
                </a>
              )}
            </div>
          </div>
        )}

        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">
            Contact Information
          </h2>

          <div className="space-y-4">
            {org.contactInfo?.email && (
              <div>
                <span className="text-sm text-stone-500">Email</span>
                <p className="font-medium">
                  <a href={`mailto:${org.contactInfo.email}`} className="text-amber-700 hover:text-amber-900">
                    {org.contactInfo.email}
                  </a>
                </p>
              </div>
            )}
            {org.contactInfo?.phone && (
              <div>
                <span className="text-sm text-stone-500">Phone</span>
                <p className="font-medium">
                  <a href={`tel:${org.contactInfo.phone}`} className="text-amber-700 hover:text-amber-900">
                    {org.contactInfo.phone}
                  </a>
                </p>
              </div>
            )}
            {org.location && (
              <div>
                <span className="text-sm text-stone-500">Location</span>
                <p className="font-medium">{org.location}</p>
              </div>
            )}
          </div>
        </div>

        {org.tags && org.tags.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">Tags</h2>
            <div className="flex flex-wrap gap-2">
              {org.tags.map((tag, idx) => (
                <span
                  key={tag.id || idx}
                  className="px-3 py-1 bg-stone-100 text-stone-700 rounded-full text-sm"
                >
                  {tag.kind}: {tag.value}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
