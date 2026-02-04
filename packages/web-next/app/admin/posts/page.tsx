"use client";

import { useState } from "react";
import { useGraphQL, graphqlMutateClient, invalidateAllMatchingQuery } from "@/lib/graphql/client";
import { GET_PENDING_POSTS } from "@/lib/graphql/queries";
import { APPROVE_POST, REJECT_POST } from "@/lib/graphql/mutations";
import { useCursorPagination } from "@/lib/hooks/useCursorPagination";
import { PaginationControls } from "@/components/ui/PaginationControls";
import type { Post, GetListingsResult } from "@/lib/types";

export default function PostsPage() {
  const [selectedPost, setSelectedPost] = useState<Post | null>(null);
  const [isApproving, setIsApproving] = useState<string | null>(null);
  const [isRejecting, setIsRejecting] = useState<string | null>(null);
  const pagination = useCursorPagination({ pageSize: 20 });

  const { data, isLoading, error, mutate: refetch } = useGraphQL<GetListingsResult>(
    GET_PENDING_POSTS,
    pagination.variables,
    { revalidateOnFocus: false }
  );

  const posts = data?.listings?.nodes || [];
  const totalCount = data?.listings?.totalCount || 0;
  const pageInfo = data?.listings?.pageInfo || { hasNextPage: false };
  const fullPageInfo = pagination.buildPageInfo(
    pageInfo.hasNextPage,
    pageInfo.startCursor,
    pageInfo.endCursor
  );

  const handleApprove = async (postId: string) => {
    if (!confirm("Approve this post? It will become visible to all users.")) return;

    setIsApproving(postId);
    try {
      await graphqlMutateClient(APPROVE_POST, { listingId: postId });
      invalidateAllMatchingQuery(GET_PENDING_POSTS);
      refetch();
      setSelectedPost(null);
    } catch (err) {
      console.error("Failed to approve:", err);
      alert("Failed to approve post");
    } finally {
      setIsApproving(null);
    }
  };

  const handleReject = async (postId: string) => {
    const reason = prompt("Reason for rejection (optional):");
    if (reason === null) return;

    setIsRejecting(postId);
    try {
      await graphqlMutateClient(REJECT_POST, {
        listingId: postId,
        reason: reason || "Rejected by admin",
      });
      invalidateAllMatchingQuery(GET_PENDING_POSTS);
      refetch();
      setSelectedPost(null);
    } catch (err) {
      console.error("Failed to reject:", err);
      alert("Failed to reject post");
    } finally {
      setIsRejecting(null);
    }
  };

  if (isLoading && posts.length === 0) {
    return <div className="p-8">Loading...</div>;
  }

  if (error) {
    return <div className="p-8 text-red-600">Error: {error.message}</div>;
  }

  return (
    <div className="max-w-7xl mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">Post Approval Queue</h1>

      {posts.length === 0 ? (
        <div className="text-stone-500 text-center py-12">No pending posts to review</div>
      ) : (
        <>
          <div className="grid gap-6 mb-6">
            {posts.map((post) => (
              <div
                key={post.id}
                className="bg-white border border-stone-200 rounded-lg p-6 hover:shadow-lg transition-shadow"
              >
                <div className="flex items-start justify-between mb-4">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-2">
                      <span className="text-xs font-medium px-2 py-1 bg-stone-100 rounded">
                        {post.submissionType === "USER_SUBMITTED"
                          ? "\u{1F464} User"
                          : "\u{1F310} Scraped"}
                      </span>
                      {post.urgency && (
                        <span
                          className={`text-xs font-medium px-2 py-1 rounded ${
                            post.urgency === "urgent"
                              ? "bg-red-100 text-red-700"
                              : post.urgency === "low"
                                ? "bg-amber-100 text-amber-700"
                                : "bg-yellow-100 text-yellow-700"
                          }`}
                        >
                          {post.urgency}
                        </span>
                      )}
                    </div>
                    <h3 className="text-xl font-semibold mb-1">{post.title}</h3>
                    <p className="text-sm text-stone-600 mb-2">{post.organizationName}</p>
                    {post.location && (
                      <p className="text-sm text-stone-500 mb-2">
                        {"\u{1F4CD}"} {post.location}
                      </p>
                    )}
                    <p className="text-stone-700 mb-4">{post.tldr}</p>
                  </div>
                </div>

                <div className="flex gap-2">
                  <button
                    onClick={() => setSelectedPost(post)}
                    className="px-4 py-2 bg-amber-600 text-white rounded hover:bg-amber-700"
                  >
                    View Details
                  </button>
                  <button
                    onClick={() => handleApprove(post.id)}
                    disabled={isApproving === post.id}
                    className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
                  >
                    {isApproving === post.id ? "..." : "\u2713 Approve"}
                  </button>
                  <button
                    onClick={() => handleReject(post.id)}
                    disabled={isRejecting === post.id}
                    className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
                  >
                    {isRejecting === post.id ? "..." : "\u2717 Reject"}
                  </button>
                </div>
              </div>
            ))}
          </div>

          <PaginationControls
            pageInfo={fullPageInfo}
            totalCount={totalCount}
            currentPage={pagination.currentPage}
            pageSize={pagination.pageSize}
            onNextPage={() => pagination.goToNextPage(pageInfo.endCursor ?? null)}
            onPreviousPage={pagination.goToPreviousPage}
            loading={isLoading}
          />
        </>
      )}

      {/* Detail Modal */}
      {selectedPost && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50">
          <div className="bg-white rounded-lg max-w-2xl w-full max-h-[80vh] overflow-y-auto p-6">
            <div className="flex justify-between items-start mb-4">
              <h2 className="text-2xl font-bold">{selectedPost.title}</h2>
              <button
                onClick={() => setSelectedPost(null)}
                className="text-stone-500 hover:text-stone-700"
              >
                {"\u2715"}
              </button>
            </div>

            <div className="space-y-4">
              <div>
                <h3 className="font-semibold text-stone-700">Organization</h3>
                <p>{selectedPost.organizationName}</p>
              </div>

              {selectedPost.location && (
                <div>
                  <h3 className="font-semibold text-stone-700">Location</h3>
                  <p>{selectedPost.location}</p>
                </div>
              )}

              <div>
                <h3 className="font-semibold text-stone-700">Description</h3>
                <p className="whitespace-pre-wrap">{selectedPost.description}</p>
              </div>

              {selectedPost.sourceUrl && (
                <div>
                  <h3 className="font-semibold text-stone-700">Source</h3>
                  <a
                    href={selectedPost.sourceUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-amber-600 hover:underline"
                  >
                    {selectedPost.sourceUrl}
                  </a>
                </div>
              )}
            </div>

            <div className="flex gap-2 mt-6">
              <button
                onClick={() => handleApprove(selectedPost.id)}
                disabled={isApproving === selectedPost.id}
                className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
              >
                {isApproving === selectedPost.id ? "..." : "\u2713 Approve"}
              </button>
              <button
                onClick={() => handleReject(selectedPost.id)}
                disabled={isRejecting === selectedPost.id}
                className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 disabled:opacity-50"
              >
                {isRejecting === selectedPost.id ? "..." : "\u2717 Reject"}
              </button>
              <button
                onClick={() => setSelectedPost(null)}
                className="px-4 py-2 bg-stone-300 text-stone-700 rounded hover:bg-stone-400"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
