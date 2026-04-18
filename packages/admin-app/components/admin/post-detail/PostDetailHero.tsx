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
import { ArrowLeft, ExternalLink } from "lucide-react";
import Link from "next/link";
import { POST_TYPES, WEIGHTS } from "@/lib/post-form-constants";
import { PencilMarkPicker } from "./PencilMarkPicker";

type PencilMark = "star" | "heart" | "smile" | "circle" | null;

type HeroPost = {
  id: string;
  title: string;
  status: string;
  postType?: string | null;
  weight?: string | null;
  priority?: number | null;
  isUrgent?: boolean | null;
  pencilMark?: string | null;
  sourceLanguage?: string | null;
  submissionType?: string | null;
  submittedBy?: { submitterType?: string | null } | null;
  createdAt: string;
  publishedAt?: string | null;
  updatedAt: string;
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
            {post.status === "active" && (
              <Link
                href={`/posts/${post.id}`}
                className="p-2 text-muted-foreground hover:text-foreground hover:bg-accent rounded-lg"
                title="View public page"
              >
                <ExternalLink className="w-4 h-4" />
              </Link>
            )}
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
        <h1 className="text-3xl font-semibold text-foreground leading-tight mb-4 tracking-tight">
          {post.title}
        </h1>

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
