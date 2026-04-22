"use client";

import * as React from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import { ArrowLeft, ExternalLink, Eye } from "lucide-react";
import Link from "next/link";
import { POST_TYPES, WEIGHTS } from "@/lib/post-form-constants";
import { PencilMarkPicker } from "./PencilMarkPicker";
import { SeedBadgeIf } from "@/components/admin/SeedBadge";

type PencilMark = "star" | "heart" | "smile" | "circle" | null;

type EditionSlotting = {
  editionId: string;
  countyId: string;
  countyName: string;
  periodStart: string;
  periodEnd: string;
  editionStatus: string;
  editionTitle?: string | null;
  slotId: string;
  postTemplate?: string | null;
};

type HeroPost = {
  id: string;
  title: string;
  status: string;
  postType?: string | null;
  weight?: string | null;
  priority?: number | null;
  isUrgent?: boolean | null;
  isSeed?: boolean | null;
  pencilMark?: string | null;
  sourceLanguage?: string | null;
  submissionType?: string | null;
  submittedBy?: { submitterType?: string | null } | null;
  createdAt: string;
  publishedAt?: string | null;
  updatedAt: string;
  editionSlottings?: EditionSlotting[] | null;
};

// submission_type values are constrained at the DB level to this set
// (see migration 213). `scraped` was the pre-213 name for `ingested`
// and was renamed in-place by the migration — no rows carry it anymore.
const SUBMISSION_LABEL: Record<string, string> = {
  admin: "Created by editor",
  ingested: "Ingested by Root Signal",
  org_submitted: "Submitted by org",
  reader_submitted: "Submitted by reader",
  revision: "Post revision",
  // Legacy compat for the `submittedBy.submitterType === 'member'`
  // fallback below; not a DB enum value.
  member: "Submitted by member",
};

function submissionLine(post: HeroPost): string | null {
  // Prefer explicit submittedBy when it's a member
  if (post.submittedBy?.submitterType === "member") return SUBMISSION_LABEL.member;
  if (post.submissionType && SUBMISSION_LABEL[post.submissionType]) {
    return SUBMISSION_LABEL[post.submissionType];
  }
  return post.submissionType ?? null;
}

function statusBadgeVariant(status: string): "success" | "warning" | "danger" | "info" | "secondary" {
  switch (status) {
    case "active": return "success";
    case "pending_approval": return "warning";
    case "draft": return "info";
    case "rejected": return "danger";
    case "archived": return "secondary";
    default: return "secondary";
  }
}

function editionStatusVariant(status: string): "success" | "warning" | "info" | "secondary" {
  // Edition status lifecycle: draft → pending_review → approved → published
  // → archived. Show published as success; in-flight as info/warning.
  switch (status) {
    case "published": return "success";
    case "approved": return "info";
    case "pending_review": return "warning";
    case "draft": return "secondary";
    case "archived": return "secondary";
    default: return "secondary";
  }
}

function formatPeriodRange(periodStart: string, periodEnd: string): string {
  // Inputs are YYYY-MM-DD dates. Render as "Apr 12–18" when in the same
  // month, else "Apr 28–May 4". Year is dropped for compactness; the
  // tooltip carries the full dates.
  const start = new Date(periodStart + "T00:00:00");
  const end = new Date(periodEnd + "T00:00:00");
  const sameMonth =
    start.getFullYear() === end.getFullYear() &&
    start.getMonth() === end.getMonth();
  const mo = (d: Date) => d.toLocaleDateString("en-US", { month: "short" });
  if (sameMonth) {
    return `${mo(start)} ${start.getDate()}–${end.getDate()}`;
  }
  return `${mo(start)} ${start.getDate()}–${mo(end)} ${end.getDate()}`;
}

function formatRelativeDate(dateString: string): { short: string; full: string } {
  const d = new Date(dateString);
  const now = Date.now();
  const diffMs = now - d.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffMin = Math.floor(diffMs / (1000 * 60));

  let short: string;
  if (diffMin < 60) short = diffMin <= 1 ? "just now" : `${diffMin}m ago`;
  else if (diffHours < 24) short = `${diffHours}h ago`;
  else if (diffDays < 7) short = `${diffDays}d ago`;
  else if (diffDays < 30) short = `${Math.floor(diffDays / 7)}w ago`;
  else if (diffDays < 365) short = d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  else short = d.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });

  const full = d.toLocaleString();
  return { short, full };
}

/**
 * "View" button next to the hero's top-right actions.
 *
 * Routes to either the public post URL or the admin preview URL:
 *   - Post is `active` AND at least one of its slotted editions is
 *     `published`        →  /posts/[id] (public)
 *   - Anything else       →  /preview/posts/[id] (admin-gated, any status)
 *
 * The button is always shown regardless of status so editors can get
 * to a preview of drafts, pending_approval posts, etc. — the old
 * behavior hid it for non-active statuses, which is exactly when the
 * preview is most useful.
 */
function PublicOrPreviewLink({ post }: { post: HeroPost }) {
  const WEB_APP_URL =
    process.env.NEXT_PUBLIC_WEB_APP_URL || "http://localhost:3001";

  const hasPublishedSlot = (post.editionSlottings ?? []).some(
    (s) => s.editionStatus === "published",
  );
  const isPubliclyVisible = post.status === "active" && hasPublishedSlot;

  const href = isPubliclyVisible
    ? `${WEB_APP_URL}/posts/${post.id}`
    : `${WEB_APP_URL}/preview/posts/${post.id}`;

  const title = isPubliclyVisible ? "View public page" : "Preview (admin only)";
  const Icon = isPubliclyVisible ? ExternalLink : Eye;

  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
      title={title}
      aria-label={title}
    >
      <Icon className="w-4 h-4" />
    </a>
  );
}

export function PostDetailHero({
  post,
  actionInProgress,
  onStatusChange,
  onDelete,
  inlineUpdate,
}: {
  post: HeroPost;
  actionInProgress: string | null;
  onStatusChange: (status: string) => void;
  onDelete: () => void;
  inlineUpdate: (input: Record<string, unknown>) => Promise<unknown>;
}) {
  const isUrgent = post.isUrgent ?? false;
  const pencilMark = (post.pencilMark ?? null) as PencilMark;

  const submitted = formatRelativeDate(post.createdAt);
  const edited = formatRelativeDate(post.updatedAt);
  const published = post.publishedAt ? formatRelativeDate(post.publishedAt) : null;

  return (
    <header className="bg-white border-b border-border">
      <div className="max-w-7xl mx-auto px-4 py-6">
        {/* Top strip: Back (left) · Delete (right) */}
        <div className="flex items-center justify-between mb-5">
          <Link
            href="/admin/posts"
            className="inline-flex items-center text-muted-foreground hover:text-foreground text-sm"
          >
            <ArrowLeft className="w-4 h-4 mr-1" /> Back to Posts
          </Link>
          <div className="flex items-center gap-2">
            <PublicOrPreviewLink post={post} />
            <Button
              variant="destructive"
              size="sm"
              onClick={onDelete}
              disabled={actionInProgress !== null}
            >
              {actionInProgress === "delete" ? "Deleting..." : "Delete"}
            </Button>
          </div>
        </div>

        {/* Title */}
        <div className="flex items-start gap-3 mb-4">
          <h1 className="text-3xl font-semibold text-foreground leading-tight tracking-tight">
            {post.title}
          </h1>
          <SeedBadgeIf isSeed={post.isSeed} className="mt-2" />
        </div>

        {/* Meta strip: status · language · date strip */}
        <div className="flex items-center flex-wrap gap-3 mb-6">
          <Select
            value={post.status}
            disabled={actionInProgress !== null}
            onValueChange={(val) => {
              if (!val || val === post.status) return;
              onStatusChange(val);
            }}
          >
            <SelectTrigger className="h-7 w-auto min-w-0 gap-1 rounded-full px-2.5 text-xs font-medium">
              <Badge variant={statusBadgeVariant(post.status)} className="pointer-events-none">
                <SelectValue />
              </Badge>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="draft">Draft</SelectItem>
              <SelectItem value="active">Active</SelectItem>
              <SelectItem value="rejected">Rejected</SelectItem>
              <SelectItem value="archived">Archived</SelectItem>
            </SelectContent>
          </Select>

          {post.sourceLanguage && (
            <Badge variant="secondary" className="text-[10px] uppercase">{post.sourceLanguage}</Badge>
          )}

          <div className="text-xs text-muted-foreground flex items-center gap-3 flex-wrap">
            {submissionLine(post) && (
              <span className="text-foreground/70">{submissionLine(post)}</span>
            )}
            <span title={submitted.full}>
              <span className="text-muted-foreground/60">Submitted </span>
              {submitted.short}
            </span>
            {published && (
              <span title={published.full}>
                <span className="text-muted-foreground/60">Published </span>
                {published.short}
              </span>
            )}
            <span title={edited.full}>
              <span className="text-muted-foreground/60">Edited </span>
              {edited.short}
            </span>
          </div>
        </div>

        {/* Editions this post is slotted into — parent-relationship
         * view. "In Editions" mirrors the "Notes" section's visual
         * language so editors reading top-to-bottom see consistent
         * chrome for "what references this post." Drafts suppress the
         * empty state (of course a draft isn't slotted). */}
        {(() => {
          const slottings = post.editionSlottings ?? [];
          if (slottings.length === 0) {
            if (post.status === "draft") return null;
            return (
              <div className="mb-6 text-xs italic text-muted-foreground">
                Not slotted into any edition yet.
              </div>
            );
          }
          return (
            <div className="mb-6">
              <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
                In Editions ({slottings.length})
              </div>
              <div className="flex flex-wrap gap-1.5">
                {slottings.map((s) => (
                  <Link
                    key={s.slotId}
                    href={`/admin/editions/${s.editionId}`}
                    className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md border border-border bg-secondary/30 hover:bg-secondary/60 transition-colors text-xs"
                    title={`${s.countyName} · ${s.periodStart} to ${s.periodEnd}`}
                  >
                    <span className="font-medium text-foreground">
                      {s.countyName}
                    </span>
                    <span className="text-muted-foreground">
                      {formatPeriodRange(s.periodStart, s.periodEnd)}
                    </span>
                    <Badge
                      variant={editionStatusVariant(s.editionStatus)}
                      className="text-[9px] px-1 py-0"
                    >
                      {s.editionStatus.replace(/_/g, " ")}
                    </Badge>
                  </Link>
                ))}
              </div>
            </div>
          );
        })()}

        {/* Broadsheet Display */}
        <div className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-3">
          Broadsheet Display
        </div>
        <div className="flex flex-wrap items-end gap-4">
          <div className="min-w-[140px]">
            <label className="block text-[10px] text-muted-foreground uppercase mb-1">Type</label>
            <Select
              value={post.postType || "story"}
              onValueChange={(v) => inlineUpdate({ postType: v })}
            >
              <SelectTrigger className="text-sm h-9 w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {POST_TYPES.map((t) => (
                  <SelectItem key={t.value} value={t.value}>{t.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="min-w-[120px]">
            <label className="block text-[10px] text-muted-foreground uppercase mb-1">Weight</label>
            <Select
              value={post.weight || "medium"}
              onValueChange={(v) => inlineUpdate({ weight: v })}
            >
              <SelectTrigger className="text-sm h-9 w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {WEIGHTS.map((w) => (
                  <SelectItem key={w.value} value={w.value}>{w.label}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="w-24">
            <label className="block text-[10px] text-muted-foreground uppercase mb-1">Priority</label>
            <Input
              type="number"
              defaultValue={post.priority ?? 0}
              className="text-sm h-9"
              onBlur={(e) => {
                const val = Number(e.target.value);
                if (val !== (post.priority ?? 0)) inlineUpdate({ priority: val });
              }}
            />
          </div>

          <label className="flex items-center gap-2 text-sm h-9 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={isUrgent}
              onChange={(e) => inlineUpdate({ isUrgent: e.target.checked })}
              className="rounded border-border"
            />
            <span className={isUrgent ? "text-red-600 font-medium" : "text-foreground"}>
              Urgent
            </span>
          </label>

          <div>
            <label className="block text-[10px] text-muted-foreground uppercase mb-1">Pencil</label>
            <PencilMarkPicker
              value={pencilMark}
              onChange={(next) => inlineUpdate({ pencilMark: next })}
            />
          </div>
        </div>
      </div>
    </header>
  );
}
