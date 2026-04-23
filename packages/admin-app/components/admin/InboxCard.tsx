"use client";

import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Check, ExternalLink, GitMerge, Pencil, X } from "lucide-react";

export interface InboxCardPost {
  id: string;
  title: string;
  bodyRaw: string;
  bodyLight?: string | null;
  postType?: string | null;
  weight?: string | null;
  isUrgent?: boolean | null;
  location?: string | null;
  createdAt: string;
  publishedAt?: string | null;
  organizationName?: string | null;
  duplicateOfId?: string | null;
  sourceUrl?: string | null;
  tags: Array<{
    id: string;
    kind: string;
    value: string;
    displayName?: string | null;
    color?: string | null;
  }>;
  meta?: {
    kicker?: string | null;
    byline?: string | null;
    deck?: string | null;
  } | null;
}

interface InboxCardProps {
  post: InboxCardPost;
  reviewFlags: string[];
  selected: boolean;
  onToggleSelect: () => void;
  onApprove: () => void;
  onReject: () => void;
  onMerge?: () => void;
  busy?: boolean;
}

const TYPE_BADGE_STYLES: Record<string, string> = {
  story: "bg-indigo-100 text-indigo-800",
  update: "bg-amber-100 text-amber-800",
  action: "bg-orange-100 text-orange-800",
  event: "bg-green-100 text-green-800",
  need: "bg-red-100 text-red-800",
  aid: "bg-emerald-100 text-emerald-800",
  person: "bg-purple-100 text-purple-800",
  business: "bg-blue-100 text-blue-800",
  reference: "bg-muted text-muted-foreground",
};

const WEIGHT_BADGE_STYLES: Record<string, string> = {
  heavy: "bg-red-100 text-red-700",
  medium: "bg-yellow-100 text-yellow-700",
  light: "bg-sky-100 text-sky-700",
};

function truncate(text: string, n: number): string {
  if (!text) return "";
  return text.length > n ? `${text.slice(0, n).trim()}…` : text;
}

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diffMs = now - then;
  const diffMin = Math.max(0, Math.floor(diffMs / 60000));
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.floor(diffHr / 24);
  if (diffDay < 7) return `${diffDay}d ago`;
  return new Date(dateStr).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
  });
}

export function InboxCard({
  post,
  reviewFlags,
  selected,
  onToggleSelect,
  onApprove,
  onReject,
  onMerge,
  busy,
}: InboxCardProps) {
  const preview = post.bodyLight?.trim() || truncate(post.bodyRaw || "", 220);
  const canMerge = reviewFlags.includes("possible_duplicate") && !!onMerge;

  return (
    <div className="bg-card border border-border rounded-lg shadow-sm hover:shadow-md transition-shadow">
      <div className="flex">
        {/* Selection checkbox */}
        <div className="pt-4 pl-4">
          <Checkbox
            checked={selected}
            onCheckedChange={onToggleSelect}
            aria-label={`Select ${post.title}`}
          />
        </div>

        {/* Left: extracted payload */}
        <div className="flex-1 p-4 pr-2 min-w-0">
          <div className="flex items-start gap-2 mb-2">
            {post.postType && (
              <span
                className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                  TYPE_BADGE_STYLES[post.postType] ?? "bg-secondary text-muted-foreground"
                }`}
              >
                {post.postType}
              </span>
            )}
            {post.weight && (
              <span
                className={`px-2 py-0.5 text-xs rounded-full font-medium ${
                  WEIGHT_BADGE_STYLES[post.weight] ?? "bg-secondary text-muted-foreground"
                }`}
              >
                {post.weight}
              </span>
            )}
            {post.isUrgent && <Badge variant="danger">Urgent</Badge>}
            {reviewFlags.map((f) => (
              <Badge key={f} variant="warning" className="font-mono text-[10px]">
                {f}
              </Badge>
            ))}
          </div>

          <Link
            href={`/admin/posts/${post.id}`}
            className="block text-base font-semibold text-foreground hover:text-admin-accent transition-colors leading-tight"
          >
            {post.title}
          </Link>

          <div className="mt-1 text-xs text-muted-foreground flex items-center gap-2 flex-wrap">
            {post.organizationName && <span>{post.organizationName}</span>}
            {post.location && (
              <>
                <span>·</span>
                <span>{post.location}</span>
              </>
            )}
            <span>·</span>
            <span>{timeAgo(post.createdAt)}</span>
          </div>

          {post.meta?.deck && (
            <p className="mt-2 text-sm text-foreground italic">{post.meta.deck}</p>
          )}

          {preview && (
            <p className="mt-2 text-sm text-muted-foreground line-clamp-3">{preview}</p>
          )}

          {post.tags?.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {post.tags.slice(0, 6).map((t) => (
                <span
                  key={t.id}
                  className="text-[10px] bg-muted text-muted-foreground px-1.5 py-0.5 rounded-full"
                >
                  {t.kind}:{t.displayName ?? t.value}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* Right: source / actions */}
        <div className="w-64 shrink-0 border-l border-border p-4 flex flex-col gap-2 text-xs">
          <div className="text-muted-foreground uppercase tracking-wider font-medium">
            Source
          </div>
          {post.sourceUrl ? (
            <a
              href={post.sourceUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 text-admin-accent hover:underline break-all"
            >
              <ExternalLink className="w-3 h-3 shrink-0" />
              <span className="truncate">{post.sourceUrl}</span>
            </a>
          ) : (
            <span className="text-muted-foreground italic">—</span>
          )}

          {post.meta?.byline && (
            <div>
              <span className="text-muted-foreground">Byline: </span>
              {post.meta.byline}
            </div>
          )}

          {post.duplicateOfId && (
            <div>
              <span className="text-muted-foreground">Duplicate of: </span>
              <Link
                href={`/admin/posts/${post.duplicateOfId}`}
                className="text-admin-accent hover:underline font-mono"
              >
                {post.duplicateOfId.slice(0, 8)}
              </Link>
            </div>
          )}

          <div className="mt-auto pt-3 flex flex-col gap-1.5">
            <Button
              size="sm"
              variant="admin"
              onClick={onApprove}
              disabled={busy}
              className="w-full"
            >
              <Check className="w-3.5 h-3.5 mr-1" />
              Approve
            </Button>
            <div className="flex gap-1.5">
              <Button
                size="sm"
                variant="outline"
                disabled={busy}
                className="flex-1"
                render={<Link href={`/admin/posts/${post.id}`} />}
              >
                <Pencil className="w-3.5 h-3.5 mr-1" />
                Edit
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={onReject}
                disabled={busy}
                className="flex-1 text-red-700 hover:text-red-800 hover:bg-red-50"
              >
                <X className="w-3.5 h-3.5 mr-1" />
                Reject
              </Button>
            </div>
            {canMerge && (
              <Button
                size="sm"
                variant="outline"
                onClick={onMerge}
                disabled={busy}
                className="w-full"
              >
                <GitMerge className="w-3.5 h-3.5 mr-1" />
                Review merge
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
