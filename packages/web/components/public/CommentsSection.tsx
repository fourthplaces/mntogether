"use client";

import { useState, FormEvent } from "react";
import { useRestateObject, callObject, invalidateObject } from "@/lib/restate/client";
import type { CommentListResult, CommentMessage } from "@/lib/restate/types";

// ---------------------------------------------------------------------------
// Tree builder
// ---------------------------------------------------------------------------

interface CommentNode {
  comment: CommentMessage;
  children: CommentNode[];
}

function buildCommentTree(comments: CommentMessage[]): CommentNode[] {
  const map = new Map<string, CommentNode>();
  const roots: CommentNode[] = [];

  for (const c of comments) {
    map.set(c.id, { comment: c, children: [] });
  }

  for (const c of comments) {
    const node = map.get(c.id)!;
    if (c.parent_message_id) {
      const parent = map.get(c.parent_message_id);
      if (parent) {
        parent.children.push(node);
        continue;
      }
    }
    roots.push(node);
  }

  return roots;
}

// ---------------------------------------------------------------------------
// Time formatting
// ---------------------------------------------------------------------------

function timeAgo(dateString: string): string {
  const diff = Date.now() - new Date(dateString).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return `${Math.floor(days / 7)}w ago`;
}

// ---------------------------------------------------------------------------
// CommentForm
// ---------------------------------------------------------------------------

function CommentForm({
  postId,
  parentMessageId,
  onSuccess,
  onCancel,
}: {
  postId: string;
  parentMessageId?: string;
  onSuccess: () => void;
  onCancel?: () => void;
}) {
  const [content, setContent] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = content.trim();
    if (!trimmed) return;

    setSubmitting(true);
    setError(null);

    try {
      await callObject("Post", postId, "add_comment", {
        content: trimmed,
        parent_message_id: parentMessageId ?? null,
      });
      setContent("");
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to post comment");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-2">
      <textarea
        value={content}
        onChange={(e) => setContent(e.target.value)}
        placeholder={parentMessageId ? "Write a reply…" : "Add a comment…"}
        rows={parentMessageId ? 2 : 3}
        className="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm text-gray-800 placeholder:text-gray-400 focus:border-blue-400 focus:outline-none focus:ring-1 focus:ring-blue-400 resize-none"
        disabled={submitting}
      />
      {error && <p className="text-xs text-red-600">{error}</p>}
      <div className="flex items-center gap-2">
        <button
          type="submit"
          disabled={submitting || !content.trim()}
          className="px-3 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {submitting ? "Posting…" : parentMessageId ? "Reply" : "Comment"}
        </button>
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-sm text-gray-500 hover:text-gray-700"
          >
            Cancel
          </button>
        )}
      </div>
    </form>
  );
}

// ---------------------------------------------------------------------------
// CommentThread (recursive)
// ---------------------------------------------------------------------------

const MAX_VISUAL_DEPTH = 4;

function CommentThread({
  node,
  postId,
  depth,
  onRefresh,
}: {
  node: CommentNode;
  postId: string;
  depth: number;
  onRefresh: () => void;
}) {
  const [replying, setReplying] = useState(false);
  const { comment, children } = node;

  return (
    <div className={depth > 0 && depth <= MAX_VISUAL_DEPTH ? "ml-6 border-l-2 border-gray-100 pl-4" : ""}>
      <div className="py-3">
        <div className="flex items-center gap-2 mb-1">
          <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-gray-100 text-xs font-medium text-gray-500">
            ?
          </span>
          <span className="text-xs text-gray-400">{timeAgo(comment.created_at)}</span>
        </div>
        <p className="text-sm text-gray-800 whitespace-pre-wrap">{comment.content}</p>
        <button
          type="button"
          onClick={() => setReplying(!replying)}
          className="mt-1 text-xs text-gray-400 hover:text-blue-600 transition-colors"
        >
          Reply
        </button>
        {replying && (
          <div className="mt-2">
            <CommentForm
              postId={postId}
              parentMessageId={comment.id}
              onSuccess={() => {
                setReplying(false);
                onRefresh();
              }}
              onCancel={() => setReplying(false)}
            />
          </div>
        )}
      </div>
      {children.map((child) => (
        <CommentThread
          key={child.comment.id}
          node={child}
          postId={postId}
          depth={Math.min(depth + 1, MAX_VISUAL_DEPTH)}
          onRefresh={onRefresh}
        />
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// CommentsSection (main export)
// ---------------------------------------------------------------------------

export default function CommentsSection({ postId }: { postId: string }) {
  const { data, mutate } = useRestateObject<CommentListResult>(
    "Post",
    postId,
    "get_comments",
    {}
  );

  const comments = data?.messages ?? [];
  const tree = buildCommentTree(comments);

  const handleRefresh = () => {
    invalidateObject("Post", postId);
    mutate();
  };

  return (
    <div className="border-t border-gray-100 pt-6 mt-8">
      <h2 className="text-lg font-semibold text-gray-900 mb-4">
        Comments{comments.length > 0 ? ` (${comments.length})` : ""}
      </h2>

      <CommentForm postId={postId} onSuccess={handleRefresh} />

      {comments.length === 0 ? (
        <p className="text-sm text-gray-400 mt-4">No comments yet. Be the first to comment.</p>
      ) : (
        <div className="mt-4 divide-y divide-gray-50">
          {tree.map((node) => (
            <CommentThread
              key={node.comment.id}
              node={node}
              postId={postId}
              depth={0}
              onRefresh={handleRefresh}
            />
          ))}
        </div>
      )}
    </div>
  );
}
