import { useMutation } from '@apollo/client';
import { TRACK_POST_VIEW, TRACK_POST_CLICK } from '../graphql/mutations';
import { useEffect } from 'react';

interface Post {
  id: string;
  listingId: string;
  status: string;
  publishedAt?: string;
  expiresAt?: string;
  customTitle?: string;
  customDescription?: string;
  customTldr?: string;
  listing: {
    id: string;
    organizationName: string;
    title: string;
    tldr?: string;
    description: string;
    contactInfo?: {
      email?: string;
      phone?: string;
      website?: string;
    };
    location?: string;
    urgency?: string;
    createdAt: string;
  };
}

interface PostCardProps {
  post: Post;
}

export function PostCard({ post }: PostCardProps) {
  const [trackView] = useMutation(TRACK_POST_VIEW);
  const [trackClick] = useMutation(TRACK_POST_CLICK);

  // Track view when component mounts
  useEffect(() => {
    trackView({ variables: { postId: post.id } });
  }, [post.id, trackView]);

  const handleContactClick = () => {
    trackClick({ variables: { postId: post.id } });
  };

  const getUrgencyColor = (urgency?: string) => {
    switch (urgency) {
      case 'urgent':
        return 'bg-red-100 text-red-700';
      case 'high':
        return 'bg-orange-100 text-orange-700';
      case 'medium':
        return 'bg-yellow-100 text-yellow-700';
      case 'low':
        return 'bg-blue-100 text-blue-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  };

  // Use custom fields if available, otherwise fall back to listing fields
  const title = post.customTitle || post.listing.title;
  const tldr = post.customTldr || post.listing.tldr;

  return (
    <div className="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow">
      {/* Urgency Badge */}
      {post.listing.urgency && (
        <div className="mb-3">
          <span className={`inline-block px-3 py-1 rounded-full text-xs font-semibold ${getUrgencyColor(post.listing.urgency)}`}>
            {post.listing.urgency.toUpperCase()}
          </span>
        </div>
      )}

      {/* Title and Organization */}
      <h3 className="text-xl font-bold text-gray-900 mb-2">{title}</h3>
      <p className="text-sm text-gray-600 mb-3">{post.listing.organizationName}</p>

      {/* Location */}
      {post.listing.location && (
        <p className="text-sm text-gray-500 mb-3">
          📍 {post.listing.location}
        </p>
      )}

      {/* TLDR */}
      {tldr && <p className="text-gray-700 mb-4">{tldr}</p>}

      {/* Contact Info */}
      {post.listing.contactInfo && (
        <div className="space-y-2">
          {post.listing.contactInfo.email && (
            <a
              href={`mailto:${post.listing.contactInfo.email}`}
              onClick={handleContactClick}
              className="block text-blue-600 hover:text-blue-800 text-sm"
            >
              ✉️ {post.listing.contactInfo.email}
            </a>
          )}
          {post.listing.contactInfo.phone && (
            <a
              href={`tel:${post.listing.contactInfo.phone}`}
              onClick={handleContactClick}
              className="block text-blue-600 hover:text-blue-800 text-sm"
            >
              📞 {post.listing.contactInfo.phone}
            </a>
          )}
          {post.listing.contactInfo.website && (
            <a
              href={post.listing.contactInfo.website}
              target="_blank"
              rel="noopener noreferrer"
              onClick={handleContactClick}
              className="block text-blue-600 hover:text-blue-800 text-sm"
            >
              🌐 Visit Website
            </a>
          )}
        </div>
      )}
    </div>
  );
}
