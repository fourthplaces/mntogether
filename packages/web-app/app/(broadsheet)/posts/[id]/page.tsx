"use client";

/**
 * Public post detail page — /posts/[id]
 *
 * Uses the shared PostDetailView renderer. The actual layout, sidebar,
 * body rendering, related posts, etc. live in
 * components/broadsheet/PostDetailView.tsx; both this page and
 * /preview/posts/[id] feed that renderer with different queries
 * (public vs admin-gated).
 *
 * Rust's GET /Post/{id}/get uses OptionalUser so non-admins only see
 * `status='active'` non-deleted posts; admins see any post but the
 * full preview banner UX lives on the dedicated /preview route.
 */

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";
import { useQuery, useMutation } from "urql";

import { PostDetailPublicQuery, TrackPostViewMutation } from "@/lib/graphql/public";
import { isAuthenticated } from "@/lib/auth/actions";
import {
  PostDetailView,
  PostDetailSkeleton,
  PostNotFound,
} from "@/components/broadsheet/PostDetailView";

export default function PublicPostDetailPage() {
  const params = useParams();
  const postId = params.id as string;

  const [{ data, fetching: isLoading }] = useQuery({
    query: PostDetailPublicQuery,
    variables: { id: postId },
  });
  const post = data?.post;

  const [, trackView] = useMutation(TrackPostViewMutation);
  const [isAdmin, setIsAdmin] = useState(false);

  useEffect(() => {
    isAuthenticated().then(setIsAdmin);
  }, []);

  useEffect(() => {
    if (postId) {
      trackView({ postId }).catch(() => {});
    }
  }, [postId, trackView]);

  if (isLoading) return <PostDetailSkeleton />;
  if (!post) return <PostNotFound />;

  const banner = isAdmin ? (
    <div className="admin-bar">
      <div className="admin-bar__inner">
        <span className="admin-bar__badge">Admin</span>
        <span>Viewing published post</span>
        <Link href={`/admin/posts/${postId}`} className="admin-bar__link">
          Edit in CMS &rarr;
        </Link>
      </div>
    </div>
  ) : null;

  return <PostDetailView post={post} banner={banner} />;
}
