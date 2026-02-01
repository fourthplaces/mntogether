import React, { useState } from 'react';
import { useMutation } from '@apollo/client';
import { gql } from '@apollo/client';

const EDIT_AND_APPROVE_POST = gql`
  mutation EditAndApprovePost($listingId: Uuid!, $input: EditListingInput!) {
    editAndApproveListing(listingId: $listingId, input: $input) {
      id
      status
      title
      description
      tldr
    }
  }
`;

interface PostEditModalProps {
  listing: any;
  onClose: () => void;
  onSuccess: () => void;
}

const PostEditModal: React.FC<PostEditModalProps> = ({
  listing,
  onClose,
  onSuccess,
}) => {
  const post = listing; // Alias for readability

  const [formData, setFormData] = useState({
    title: post.title || '',
    description: post.description || '',
    tldr: post.tldr || '',
    urgency: post.urgency || 'medium',
    location: post.location || '',
  });

  const [editAndApprovePost, { loading, error }] = useMutation(EDIT_AND_APPROVE_POST, {
    onCompleted: () => {
      onSuccess();
      onClose();
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    editAndApprovePost({
      variables: {
        listingId: post.id,
        input: {
          title: formData.title,
          description: formData.description,
          tldr: formData.tldr || null,
          urgency: formData.urgency || null,
          location: formData.location || null,
        },
      },
    });
  };

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>
  ) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
  };

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50 overflow-y-auto">
      <div className="bg-white rounded-lg p-6 max-w-2xl w-full max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-stone-900">Edit & Approve Post</h2>
          <button
            onClick={onClose}
            className="text-stone-400 hover:text-stone-600 text-2xl leading-none"
          >
            x
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Organization Name (Read-only) */}
          <div>
            <label className="block text-sm font-medium text-stone-700 mb-1">
              Organization
            </label>
            <input
              type="text"
              value={post.organizationName}
              disabled
              className="w-full px-3 py-2 border border-stone-300 rounded bg-stone-50 text-stone-600"
            />
          </div>

          {/* Post Type (Read-only) */}
          <div>
            <label className="block text-sm font-medium text-stone-700 mb-1">
              Type
            </label>
            <input
              type="text"
              value={post.postType}
              disabled
              className="w-full px-3 py-2 border border-stone-300 rounded bg-stone-50 text-stone-600 capitalize"
            />
          </div>

          {/* Title */}
          <div>
            <label className="block text-sm font-medium text-stone-700 mb-1">
              Title <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              name="title"
              value={formData.title}
              onChange={handleChange}
              required
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
            />
          </div>

          {/* TLDR */}
          <div>
            <label className="block text-sm font-medium text-stone-700 mb-1">
              TLDR (Short Summary)
            </label>
            <input
              type="text"
              name="tldr"
              value={formData.tldr}
              onChange={handleChange}
              placeholder="1-2 sentence summary"
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
            />
            <p className="text-xs text-stone-500 mt-1">
              A brief, catchy summary (shown in cards and previews)
            </p>
          </div>

          {/* Description */}
          <div>
            <label className="block text-sm font-medium text-stone-700 mb-1">
              Description <span className="text-red-500">*</span>
            </label>
            <textarea
              name="description"
              value={formData.description}
              onChange={handleChange}
              required
              rows={6}
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
            />
            <p className="text-xs text-stone-500 mt-1">
              Full details about what's offered/needed (requirements, impact, etc.)
            </p>
          </div>

          {/* Location */}
          <div>
            <label className="block text-sm font-medium text-stone-700 mb-1">
              Location
            </label>
            <input
              type="text"
              name="location"
              value={formData.location}
              onChange={handleChange}
              placeholder="City, State"
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
            />
          </div>

          {/* Urgency */}
          <div>
            <label className="block text-sm font-medium text-stone-700 mb-1">
              Urgency
            </label>
            <select
              name="urgency"
              value={formData.urgency}
              onChange={handleChange}
              className="w-full px-3 py-2 border border-stone-300 rounded focus:outline-none focus:ring-2 focus:ring-amber-500"
            >
              <option value="">None</option>
              <option value="low">Low</option>
              <option value="medium">Medium</option>
              <option value="high">High</option>
              <option value="urgent">Urgent</option>
            </select>
          </div>

          {/* Source URL (Read-only) */}
          {post.sourceUrl && (
            <div>
              <label className="block text-sm font-medium text-stone-700 mb-1">
                Source URL
              </label>
              <a
                href={post.sourceUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="block w-full px-3 py-2 border border-stone-300 rounded bg-stone-50 text-amber-600 hover:text-amber-800 break-all"
              >
                {post.sourceUrl}
              </a>
            </div>
          )}

          {/* Error Display */}
          {error && (
            <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded">
              Error: {error.message}
            </div>
          )}

          {/* Action Buttons */}
          <div className="flex gap-3 pt-4">
            <button
              type="submit"
              disabled={loading}
              className="flex-1 px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:bg-green-400 disabled:cursor-not-allowed font-medium"
            >
              {loading ? 'Saving...' : 'Save & Approve'}
            </button>
            <button
              type="button"
              onClick={onClose}
              disabled={loading}
              className="px-6 py-2 bg-stone-200 text-stone-700 rounded hover:bg-stone-300 disabled:bg-stone-100 disabled:cursor-not-allowed font-medium"
            >
              Cancel
            </button>
          </div>
        </form>

        {/* Info Box */}
        <div className="mt-6 p-4 bg-amber-50 border border-amber-200 rounded">
          <h3 className="font-semibold text-sm text-amber-900 mb-2">
            Editing Tips
          </h3>
          <ul className="text-xs text-amber-800 space-y-1 list-disc list-inside">
            <li>Make titles clear and concise (5-10 words)</li>
            <li>TLDR should be a compelling 1-2 sentence hook</li>
            <li>Include practical details in description (requirements, impact, contact)</li>
            <li>Set urgency appropriately to help users prioritize</li>
            <li>This will approve the post and make it live immediately</li>
          </ul>
        </div>
      </div>
    </div>
  );
};

export default PostEditModal;
