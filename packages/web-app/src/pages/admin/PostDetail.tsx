import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, gql } from '@apollo/client';
import ReactMarkdown from 'react-markdown';
import { useState } from 'react';

const GET_POST = gql`
  query GetPost($id: Uuid!) {
    listing(id: $id) {
      id
      organizationName
      title
      tldr
      description
      descriptionMarkdown
      postType
      category
      urgency
      location
      status
      sourceUrl
      websiteId
      hasEmbedding
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

const GET_PAGE_SNAPSHOT_BY_URL = gql`
  query GetPageSnapshotByUrl($url: String!) {
    pageSnapshotByUrl(url: $url) {
      id
    }
  }
`;

const ADD_POST_TAG = gql`
  mutation AddPostTag($listingId: Uuid!, $tagKind: String!, $tagValue: String!, $displayName: String) {
    addListingTag(listingId: $listingId, tagKind: $tagKind, tagValue: $tagValue, displayName: $displayName) {
      id
      kind
      value
      displayName
    }
  }
`;

const REMOVE_POST_TAG = gql`
  mutation RemovePostTag($listingId: Uuid!, $tagId: String!) {
    removeListingTag(listingId: $listingId, tagId: $tagId)
  }
`;

const GENERATE_POST_EMBEDDING = gql`
  mutation GeneratePostEmbedding($postId: Uuid!) {
    generatePostEmbedding(postId: $postId)
  }
`;

const REGENERATE_PAGE_POSTS = gql`
  mutation RegeneratePagePosts($pageSnapshotId: Uuid!) {
    regeneratePagePosts(pageSnapshotId: $pageSnapshotId) {
      jobId
      status
      message
    }
  }
`;

interface Tag {
  id: string;
  kind: string;
  value: string;
  displayName: string | null;
}

interface Post {
  id: string;
  organizationName: string;
  title: string;
  tldr: string | null;
  description: string;
  descriptionMarkdown: string | null;
  postType: string;
  category: string;
  urgency: string | null;
  location: string | null;
  status: string;
  sourceUrl: string | null;
  websiteId: string | null;
  hasEmbedding: boolean;
  createdAt: string;
  tags: Tag[];
}

const AUDIENCE_ROLES = [
  { value: 'recipient', label: 'Recipient', description: 'People receiving services/benefits' },
  { value: 'donor', label: 'Donor', description: 'People giving money/goods' },
  { value: 'volunteer', label: 'Volunteer', description: 'People giving their time' },
  { value: 'participant', label: 'Participant', description: 'People attending events/groups' },
  { value: 'customer', label: 'Customer', description: 'People buying from immigrant-owned businesses' },
];

export function PostDetail() {
  const { postId } = useParams<{ postId: string }>();
  const [isEditingTags, setIsEditingTags] = useState(false);
  const [showMoreMenu, setShowMoreMenu] = useState(false);

  const { data, loading, error, refetch } = useQuery<{ listing: Post | null }>(GET_POST, {
    variables: { id: postId },
    skip: !postId,
  });

  const post = data?.listing;

  // Query for page snapshot by URL (only if post has sourceUrl)
  const { data: pageSnapshotData } = useQuery<{ pageSnapshotByUrl: { id: string } | null }>(
    GET_PAGE_SNAPSHOT_BY_URL,
    {
      variables: { url: post?.sourceUrl || '' },
      skip: !post?.sourceUrl,
    }
  );

  const pageSnapshotId = pageSnapshotData?.pageSnapshotByUrl?.id;

  const [addTag, { loading: addingTag }] = useMutation(ADD_POST_TAG, {
    onCompleted: () => refetch(),
  });

  const [removeTag, { loading: removingTag }] = useMutation(REMOVE_POST_TAG, {
    onCompleted: () => refetch(),
  });

  const [generateEmbedding, { loading: generatingEmbedding }] = useMutation(GENERATE_POST_EMBEDDING, {
    onCompleted: () => {
      refetch();
      setShowMoreMenu(false);
    },
    onError: (error) => {
      alert(`Failed to generate embedding: ${error.message}`);
    },
  });

  const [regeneratePagePosts, { loading: regeneratingPosts }] = useMutation(REGENERATE_PAGE_POSTS, {
    onCompleted: (data) => {
      setShowMoreMenu(false);
      if (data.regeneratePagePosts.status === 'queued') {
        alert('Post regeneration started. Refresh the page in a moment to see updates.');
      }
    },
    onError: (error) => {
      alert(`Failed to regenerate: ${error.message}`);
    },
  });

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
      case 'customer':
        return 'bg-teal-100 text-teal-800';
      default:
        return 'bg-stone-100 text-stone-800';
    }
  };

  const audienceRoleTags = post?.tags.filter(t => t.kind === 'audience_role') || [];
  const otherTags = post?.tags.filter(t => t.kind !== 'audience_role') || [];

  const handleToggleAudienceRole = async (role: string) => {
    if (!postId) return;

    const existingTag = audienceRoleTags.find(t => t.value === role);
    if (existingTag) {
      await removeTag({ variables: { listingId: postId, tagId: existingTag.id } });
    } else {
      const roleInfo = AUDIENCE_ROLES.find(r => r.value === role);
      await addTag({
        variables: {
          listingId: postId,
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
        <div className="text-stone-600">Loading post...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-red-600 mb-4">Error Loading Post</h1>
            <p className="text-stone-600 mb-4">{error.message}</p>
            <Link to="/admin/posts" className="text-blue-600 hover:text-blue-800">
              Back to Posts
            </Link>
          </div>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="min-h-screen bg-stone-50 p-6">
        <div className="max-w-4xl mx-auto">
          <div className="text-center py-12">
            <h1 className="text-2xl font-bold text-stone-900 mb-4">Post Not Found</h1>
            <Link to="/admin/posts" className="text-blue-600 hover:text-blue-800">
              Back to Posts
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
          to="/admin/posts"
          className="inline-flex items-center text-stone-600 hover:text-stone-900 mb-6"
        >
          <svg className="w-5 h-5 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Posts
        </Link>

        {/* Post Header */}
        <div className="bg-white rounded-lg shadow-md p-6 mb-6">
          <div className="flex justify-between items-start mb-4">
            <div className="flex-1">
              <h1 className="text-2xl font-bold text-stone-900 mb-2 select-text">{post.title}</h1>
              <p className="text-lg text-stone-600 select-text">{post.organizationName}</p>
            </div>
            <div className="flex items-center gap-2">
              <span
                className={`px-3 py-1 text-sm rounded-full font-medium ${getStatusBadgeClass(post.status)}`}
              >
                {post.status.replace('_', ' ')}
              </span>

              {/* More Menu */}
              <div className="relative">
                <button
                  onClick={() => setShowMoreMenu(!showMoreMenu)}
                  className="p-2 text-stone-400 hover:text-stone-600 hover:bg-stone-100 rounded-lg"
                  title="More actions"
                >
                  <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                  </svg>
                </button>

                {showMoreMenu && (
                  <>
                    <div
                      className="fixed inset-0 z-10"
                      onClick={() => setShowMoreMenu(false)}
                    />
                    <div className="absolute right-0 mt-2 w-56 bg-white rounded-lg shadow-lg border border-stone-200 z-20">
                      <div className="py-1">
                        <button
                          onClick={() => {
                            if (pageSnapshotId) {
                              regeneratePagePosts({ variables: { pageSnapshotId } });
                            } else {
                              alert('No page snapshot available for this post');
                              setShowMoreMenu(false);
                            }
                          }}
                          disabled={!pageSnapshotId || regeneratingPosts}
                          className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                          </svg>
                          {regeneratingPosts ? 'Regenerating...' : 'Regenerate from Source'}
                        </button>
                        <button
                          onClick={() => {
                            generateEmbedding({ variables: { postId } });
                          }}
                          disabled={post.hasEmbedding || generatingEmbedding}
                          className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                          </svg>
                          {generatingEmbedding ? 'Generating...' : (post.hasEmbedding ? 'Embedding exists' : 'Generate Embedding')}
                        </button>
                        <div className="border-t border-stone-200 my-1" />
                        <button
                          onClick={() => {
                            setShowMoreMenu(false);
                            if (post.sourceUrl) {
                              const url = post.sourceUrl.startsWith('http') ? post.sourceUrl : `https://${post.sourceUrl}`;
                              window.open(url, '_blank');
                            }
                          }}
                          disabled={!post.sourceUrl}
                          className="w-full px-4 py-2 text-left text-sm text-stone-700 hover:bg-stone-50 disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
                        >
                          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                          </svg>
                          View Source Page
                        </button>
                      </div>
                    </div>
                  </>
                )}
              </div>
            </div>
          </div>

          {/* Missing Fields Warning */}
          {(() => {
            const missingFields = [];
            if (!post.hasEmbedding) missingFields.push('embedding');
            if (!post.tldr) missingFields.push('TLDR');
            if (!post.location) missingFields.push('location');
            if (audienceRoleTags.length === 0) missingFields.push('audience role');

            if (missingFields.length > 0) {
              return (
                <div className="mb-4 p-3 bg-amber-50 border border-amber-200 rounded-lg flex items-start gap-2">
                  <svg className="w-5 h-5 text-amber-600 flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                  </svg>
                  <div>
                    <span className="text-sm font-medium text-amber-800">Missing fields: </span>
                    <span className="text-sm text-amber-700">{missingFields.join(', ')}</span>
                  </div>
                </div>
              );
            }
            return null;
          })()}

          {post.tldr && (
            <p className="text-stone-700 bg-amber-50 p-3 rounded-lg mb-4 select-text">
              {post.tldr}
            </p>
          )}

          {/* Details Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 pt-4 border-t border-stone-200">
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Type</span>
              <p className="text-sm font-medium text-stone-900">{post.postType}</p>
            </div>
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Category</span>
              <p className="text-sm font-medium text-stone-900">{post.category}</p>
            </div>
            {post.urgency && (
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Urgency</span>
                <p className="text-sm font-medium text-stone-900">{post.urgency}</p>
              </div>
            )}
            {post.location && (
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Location</span>
                <p className="text-sm font-medium text-stone-900">{post.location}</p>
              </div>
            )}
            <div className="select-text">
              <span className="text-xs text-stone-500 uppercase">Created</span>
              <p className="text-sm font-medium text-stone-900">{formatDate(post.createdAt)}</p>
            </div>
            {post.websiteId && (
              <div className="select-text">
                <span className="text-xs text-stone-500 uppercase">Website</span>
                <p className="text-sm font-medium">
                  <Link
                    to={`/admin/websites/${post.websiteId}`}
                    className="text-blue-600 hover:text-blue-800"
                  >
                    View Website →
                  </Link>
                </p>
              </div>
            )}
            {post.sourceUrl && (
              <div className="select-text col-span-2">
                <span className="text-xs text-stone-500 uppercase">Source Page</span>
                <div className="flex items-center gap-4">
                  {pageSnapshotId ? (
                    <Link
                      to={`/admin/pages/${pageSnapshotId}`}
                      className="text-blue-600 hover:text-blue-800 text-sm font-medium"
                    >
                      View Scraped Page →
                    </Link>
                  ) : (
                    <span className="text-stone-400 text-sm">No scraped page available</span>
                  )}
                  <a
                    href={post.sourceUrl.startsWith('http') ? post.sourceUrl : `https://${post.sourceUrl}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-stone-500 hover:text-stone-700 text-sm flex items-center gap-1"
                  >
                    Open original
                    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                    </svg>
                  </a>
                </div>
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
            Who is this post for? Select all that apply.
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
          {post.descriptionMarkdown ? (
            <div className="prose prose-stone max-w-none select-text">
              <ReactMarkdown>{post.descriptionMarkdown}</ReactMarkdown>
            </div>
          ) : (
            <p className="text-stone-700 whitespace-pre-wrap select-text">{post.description}</p>
          )}
        </div>
      </div>
    </div>
  );
}
