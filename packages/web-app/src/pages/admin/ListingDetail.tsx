import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, gql } from '@apollo/client';
import ReactMarkdown from 'react-markdown';
import { useState } from 'react';

const GET_LISTING = gql`
  query GetListing($id: Uuid!) {
    listing(id: $id) {
      id
      organizationName
      title
      tldr
      description
      descriptionMarkdown
      listingType
      category
      urgency
      location
      status
      sourceUrl
      createdAt
      tags {
        id
        kind
        value
        displayName
      }
    }
  }
`;

const ADD_LISTING_TAG = gql`
  mutation AddListingTag($listingId: Uuid!, $tagKind: String!, $tagValue: String!, $displayName: String) {
    addListingTag(listingId: $listingId, tagKind: $tagKind, tagValue: $tagValue, displayName: $displayName) {
      id
      kind
      value
      displayName
    }
  }
`;

const REMOVE_LISTING_TAG = gql`
  mutation RemoveListingTag($listingId: Uuid!, $tagId: String!) {
    removeListingTag(listingId: $listingId, tagId: $tagId)
  }
`;

interface Tag {
  id: string;
  kind: string;
  value: string;
  displayName: string | null;
}

interface Listing {
  id: string;
  organizationName: string;
  title: string;
  tldr: string | null;
  description: string;
  descriptionMarkdown: string | null;
  listingType: string;
  category: string;
  urgency: string | null;
  location: string | null;
  status: string;
  sourceUrl: string | null;
  createdAt: string;
  tags: Tag[];
}

const AUDIENCE_ROLES = [
  { value: 'recipient', label: 'Recipient', description: 'People receiving services/benefits' },
  { value: 'donor', label: 'Donor', description: 'People giving money/goods' },
  { value: 'volunteer', label: 'Volunteer', description: 'People giving their time' },
  { value: 'participant', label: 'Participant', description: 'People attending events/groups' },
];

export function ListingDetail() {
  const { listingId } = useParams<{ listingId: string }>();
  const [isEditingTags, setIsEditingTags] = useState(false);

  const { data, loading, error, refetch } = useQuery<{ listing: Listing | null }>(GET_LISTING, {
    variables: { id: listingId },
    skip: !listingId,
  });

  const [addTag, { loading: addingTag }] = useMutation(ADD_LISTING_TAG, {
    onCompleted: () => refetch(),
  });

  const [removeTag, { loading: removingTag }] = useMutation(REMOVE_LISTING_TAG, {
    onCompleted: () => refetch(),
  });

  const listing = data?.listing;

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString();
  };

  const getStatusBadgeClass = (status: string) => {
    switch (status) {
      case 'active':
        return 'bg-green-100 text-green-800';
      case 'pending_approval':
        return 'bg-amber-100 text-amber-800';
      case 'rejected':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-stone-100 text-stone-800';
    }
  };

  const getAudienceRoleBadgeClass = (role: string) => {
    switch (role) {
      case 'recipient':
        return 'bg-blue-100 text-blue-800';
      case 'donor':
        return 'bg-green-100 text-green-800';
      case 'volunteer':
        return 'bg-purple-100 text-purple-800';
      case 'participant':
        return 'bg-amber-100 text-amber-800';
      default:
        return 'bg-stone-100 text-stone-800';
    }
  };

  const audienceRoleTags = listing?.tags.filter(t => t.kind === 'audience_role') || [];
  const otherTags = listing?.tags.filter(t => t.kind !== 'audience_role') || [];

  const handleToggleAudienceRole = async (role: string) => {
    if (!listingId) return;

    const existingTag = audienceRoleTags.find(t => t.value === role);
    if (existingTag) {
      await removeTag({ variables: { listingId, tagId: existingTag.id } });
    } else {
      const roleInfo = AUDIENCE_ROLES.find(r => r.value === role);
      await addTag({
        variables: {
          listingId,
          tagKind: 'audience_role',
          tagValue: role,
          displayName: roleInfo?.label || role,
        },
      });
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-stone-600">Loading listing...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Listing</h1>
            <p className="text-stone-600 mb-4">{error.message}</p>
            <Link to="/admin/listings" className="text-blue-600 hover:text-blue-800">
              Back to Listings
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!listing) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Listing Not Found</h1>
            <Link to="/admin/listings" className="text-blue-600 hover:text-blue-800">
              Back to Listings
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
          to="/admin/listings"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          <svg className="w-5 h-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Listings
        </Link>

        {/* Listing Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start mb-4">
            <div className="flex-1">
              <h1 className="text-2xl font-bold text-stone-900 mb-2 select-text">{listing.title}</h1>
              <p className="text-lg text-stone-600 select-text">{listing.organizationName}</p>
            </div>
            <span
              className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusBadgeClass(listing.status)}`}
            >
              {listing.status.replace('_', ' ')}
            </span>
          </div>

          {listing.tldr && (
            <p className="text-stone-700 bg-amber-50 p-3 rounded-lg mb-4 select-text">
              {listing.tldr}
            </p>
          )}

          {/* Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-stone-200">
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Type</span>
              <p className="text-sm font-medium text-stone-900">{listing.listingType}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Category</span>
              <p className="text-sm font-medium text-stone-900">{listing.category}</p>
            </div>
            {listing.urgency && (
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Urgency</span>
                <p className="text-sm font-medium text-stone-900">{listing.urgency}</p>
              </div>
            )}
            {listing.location && (
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Location</span>
                <p className="text-sm font-medium text-stone-900">{listing.location}</p>
              </div>
            )}
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(listing.createdAt)}</p>
            </div>
            {listing.sourceUrl && (
              <div className="select-text col-span-2">
                <span className="text-xs text-stone-500 uppercase">Source URL</span>
                <p className="text-sm font-medium">
                  <a
                    href={listing.sourceUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:text-blue-800 truncate block"
                  >
                    {listing.sourceUrl}
                  </a>
                </p>
              </div>
            )}
          </div>
        </div>

        {/* Audience Roles */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-lg font-semibold text-stone-900">Audience Roles</h2>
            <button
              onClick={() => setIsEditingTags(!isEditingTags)}
              className="text-sm text-blue-600 hover:text-blue-800"
            >
              {isEditingTags ? 'Done' : 'Edit'}
            </button>
          </div>

          <p className="text-sm text-stone-500 mb-4">
            Who is this listing for? Select all that apply.
          </p>

          {isEditingTags ? (
            <div className="grid grid-cols-2 gap-3">
              {AUDIENCE_ROLES.map((role) => {
                const isSelected = audienceRoleTags.some(t => t.value === role.value);
                return (
                  <button
                    key={role.value}
                    onClick={() => handleToggleAudienceRole(role.value)}
                    disabled={addingTag || removingTag}
                    className={`p-3 rounded-lg border-2 text-left transition-colors ${
                      isSelected
                        ? 'border-blue-500 bg-blue-50'
                        : 'border-stone-200 hover:border-stone-300'
                    } ${(addingTag || removingTag) ? 'opacity-50 cursor-wait' : ''}`}
                  >
                    <div className="font-medium text-stone-900">{role.label}</div>
                    <div className="text-xs text-stone-500">{role.description}</div>
                  </button>
                );
              })}
            </div>
          ) : (
            <div className="flex flex-wrap gap-2">
              {audienceRoleTags.length > 0 ? (
                audienceRoleTags.map((tag) => (
                  <span
                    key={tag.id}
                    className={`px-3 py-1 text-sm rounded-full font-medium ${getAudienceRoleBadgeClass(tag.value)}`}
                  >
                    {tag.displayName || tag.value}
                  </span>
                ))
              ) : (
                <span className="text-stone-400 text-sm">No audience roles set</span>
              )}
            </div>
          )}
        </div>

        {/* Other Tags */}
        {otherTags.length > 0 && (
          <div className="bg-white rounded-lg shadow-md p-6 mb-6">
            <h2 className="text-lg font-semibold text-stone-900 mb-4">Other Tags</h2>
            <div className="flex flex-wrap gap-2">
              {otherTags.map((tag) => (
                <span
                  key={tag.id}
                  className="px-3 py-1 text-sm rounded-full font-medium bg-stone-100 text-stone-800"
                >
                  <span className="text-stone-500">{tag.kind}:</span> {tag.displayName || tag.value}
                </span>
              ))}
            </div>
          </div>
        )}

        {/* Description */}
        <div className="bg-white rounded-lg shadow-md p-6">
          <h2 className="text-lg font-semibold text-stone-900 mb-4">Description</h2>
          {listing.descriptionMarkdown ? (
            <div className="prose prose-stone max-w-none select-text">
              <ReactMarkdown>{listing.descriptionMarkdown}</ReactMarkdown>
            </div>
          ) : (
            <p className="text-stone-700 whitespace-pre-wrap select-text">{listing.description}</p>
          )}
        </div>
      </div>
    </div>
  );
}
