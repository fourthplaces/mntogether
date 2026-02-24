"use client";

import { useState, FormEvent } from "react";
import { useQuery, useMutation } from "urql";
import { PostDetailPublicQuery, AddCommentMutation } from "@/lib/graphql/public";

// ---------------------------------------------------------------------------
// Tree builder
// ---------------------------------------------------------------------------

interface CommentData {
  id: string;
  content: string;
  parentMessageId?: string | null;
  createdAt: string;
  role: string;
}

interface CommentNode {
  comment: CommentData;
  children: CommentNode[];
}

function buildCommentTree(comments: CommentData[]): CommentNode[] {
  const map = new Map<string, CommentNode>();
  const roots: CommentNode[] = [];

  for (const c of comments) {
    map.set(c.id, { comment: c, children: [] });
  }

  for (const c of comments) {
    const node = map.get(c.id)!;
    if (c.parentMessageId) {
      const parent = map.get(c.parentMessageId);
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
  const [, addComment] = useMutation(AddCommentMutation);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = content.trim();
    if (!trimmed) return;

    setSubmitting(true);
    setError(null);

    try {
      const result = await addComment(
        {
          postId,
          content: trimmed,
          parentMessageId: parentMessageId ?? null,
        },
        { additionalTypenames: ["Comment", "Post"] }
      );
      if (result.error) throw result.error;
      setContent("");
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to post comment");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      <div className="border border-border bg-surface-subtle overflow-hidden focus-within:border-border-strong">
        <textarea
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder={parentMessageId ? "Write a reply..." : "Share your thoughts..."}
          rows={parentMessageId ? 2 : 3}
          className="w-full bg-transparent px-4 pt-3 pb-2 text-sm text-text-primary placeholder:text-text-faint focus:outline-none resize-none"
          disabled={submitting}
        />
        <div className="flex items-center justify-between px-3 pb-2">
          <div>
            {error && <p className="text-xs text-danger-text">{error}</p>}
          </div>
          <div className="flex items-center gap-2">
            {onCancel && (
              <button
                type="button"
                onClick={onCancel}
                className="px-3 py-1 text-xs font-medium text-text-muted hover:text-text-primary"
              >
                Cancel
              </button>
            )}
            <button
              type="submit"
              disabled={submitting || !content.trim()}
              className="px-4 py-1.5 text-xs font-semibold text-text-on-action bg-action hover:bg-action-hover disabled:opacity-30 disabled:cursor-not-allowed"
            >
              {submitting ? "Posting..." : parentMessageId ? "Reply" : "Post"}
            </button>
          </div>
        </div>
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
    <div className={depth > 0 && depth <= MAX_VISUAL_DEPTH ? "ml-5 pl-4 border-l-2 border-border" : ""}>
      <div className="py-3 group">
        <p className="text-[0.9rem] text-text-primary whitespace-pre-wrap leading-relaxed">{comment.content}</p>
        <div className="flex items-center gap-3 mt-1.5 px-1">
          <span className="text-[0.7rem] text-text-faint">{timeAgo(comment.createdAt)}</span>
          <button
            type="button"
            onClick={() => setReplying(!replying)}
            className="text-[0.7rem] font-semibold text-text-faint hover:text-text-secondary"
          >
            Reply
          </button>
        </div>
        {replying && (
          <div className="mt-3">
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
  const [{ data }, reexecuteQuery] = useQuery({
    query: PostDetailPublicQuery,
    variables: { id: postId },
  });

  const comments = data?.post?.comments ?? [];
  const tree = buildCommentTree(comments);

  const handleRefresh = () => {
    reexecuteQuery({ requestPolicy: "network-only" });
  };

  return (
    <div>
      <div className="bg-surface-raised border border-border p-6">
        <h2 className="text-base font-bold text-text-primary mb-4">
          Conversation{comments.length > 0 && <span className="ml-1.5 text-xs font-medium text-text-faint">{comments.length}</span>}
        </h2>

        <CommentForm postId={postId} onSuccess={handleRefresh} />

        {comments.length === 0 ? (
          <div className="text-center py-8">
            <p className="text-sm text-text-faint">No comments yet</p>
            <p className="text-xs text-text-faint mt-1">Start the conversation above</p>
          </div>
        ) : (
          <div className="mt-5 space-y-1">
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
    </div>
  );
}
