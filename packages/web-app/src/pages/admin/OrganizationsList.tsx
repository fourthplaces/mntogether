import { useQuery } from '@apollo/client';
import { GET_CAUSE_DRIVEN_BUSINESSES } from '@/graphql/queries';
import { BusinessInfoCard } from '@/components/BusinessInfoCard';
import type { Organization } from '@/types/organization';

interface OrganizationsResponse {
  organizations: Organization[];
}

export function OrganizationsList() {
  const { data, loading, error } = useQuery<OrganizationsResponse>(GET_CAUSE_DRIVEN_BUSINESSES);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-stone-600">Loading organizations...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-amber-50">
        <div className="text-red-600">Error loading organizations: {error.message}</div>
      </div>
    );
  }

  const organizations = data?.organizations || [];
  const causeDrivenOrgs = organizations.filter(org => org.businessInfo?.isCauseDriven);

  return (
    <div className="min-h-screen bg-amber-50 p-6">
      <div className="max-w-6xl mx-auto">
        <h1 className="text-3xl font-bold text-stone-900 mb-6">Cause-Driven Businesses</h1>

        {/* Summary */}
        <div className="bg-white rounded-lg shadow-md p-4 mb-6">
          <p className="text-stone-600">
            Found <span className="font-semibold text-amber-700">{causeDrivenOrgs.length}</span>
            {' '}cause-driven business{causeDrivenOrgs.length !== 1 ? 'es' : ''} that donate proceeds to charitable causes.
          </p>
        </div>

        {/* Organizations Grid */}
        {causeDrivenOrgs.length > 0 ? (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {causeDrivenOrgs.map((org) => (
              <div key={org.id} className="bg-white rounded-lg shadow-md overflow-hidden">
                {/* Header */}
                <div className="p-4 bg-stone-50 border-b border-stone-200">
                  <h3 className="text-xl font-semibold text-stone-900">{org.name}</h3>
                  {org.verified && (
                    <span className="inline-flex items-center gap-1 text-sm text-green-700 mt-1">
                      <span>✓</span>
                      <span>Verified</span>
                    </span>
                  )}
                </div>

                {/* Description */}
                <div className="p-4">
                  <p className="text-stone-600 text-sm mb-4 line-clamp-3">
                    {org.description}
                  </p>

                  {/* Business Info Card */}
                  <BusinessInfoCard
                    businessInfo={org.businessInfo}
                    tags={org.tags}
                    organizationName={org.name}
                  />
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="bg-white rounded-lg shadow-md p-12 text-center">
            <p className="text-stone-600 mb-2">No cause-driven businesses found yet.</p>
            <p className="text-sm text-stone-500">
              Businesses that donate a percentage of proceeds to charity will appear here.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
