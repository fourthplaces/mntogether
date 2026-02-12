"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useRestateObject, callObject } from "@/lib/restate/client";
import { useEffect, useState } from "react";
import type { PostResult, TagResult, PostScheduleResult } from "@/lib/restate/types";
import { isAuthenticated } from "@/lib/auth/actions";
import { BackLink } from "@/components/ui/BackLink";
import CommentsSection from "@/components/public/CommentsSection";

function formatCategory(value: string): string {
  return value
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

const DAY_NAMES = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

function formatTime12h(time24: string): string {
  const [h, m] = time24.split(":").map(Number);
  const suffix = h >= 12 ? "PM" : "AM";
  const h12 = h % 12 || 12;
  return `${h12}:${m.toString().padStart(2, "0")} ${suffix}`;
}

function formatSchedule(s: PostScheduleResult): string {
  // One-off event: has dtstart, no day_of_week
  if (s.dtstart && s.day_of_week == null) {
    const date = new Date(s.dtstart);
    const dateStr = date.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
    const timeStr = s.opens_at && s.closes_at
      ? `${formatTime12h(s.opens_at)} – ${formatTime12h(s.closes_at)}`
      : s.opens_at ? formatTime12h(s.opens_at) : "";
    const parts = [dateStr, timeStr].filter(Boolean).join("  ");
    return s.notes ? `${parts} (${s.notes})` : parts;
  }

  // Recurring: has day_of_week
  const dayName = s.day_of_week != null ? DAY_NAMES[s.day_of_week] : "";
  const timeStr = s.opens_at && s.closes_at
    ? `${formatTime12h(s.opens_at)} – ${formatTime12h(s.closes_at)}`
    : s.opens_at ? formatTime12h(s.opens_at) : "";

  let suffix = "";
  if (s.rrule?.includes("INTERVAL=2")) suffix = " (every other week)";
  if (s.rrule?.includes("FREQ=MONTHLY")) suffix = " (monthly)";
  if (s.notes) suffix = ` (${s.notes})`;

  return [dayName, timeStr, suffix].filter(Boolean).join("  ");
}

function isScheduleExpired(s: PostScheduleResult): boolean {
  if (s.dtend && !s.rrule) return new Date(s.dtend) < new Date();
  if (s.dtstart && !s.rrule && !s.dtend) return new Date(s.dtstart) < new Date();
  return false;
}

function formatTimeAgo(dateString: string) {
  const date = new Date(dateString);
  const now = new Date();
  const diffInDays = Math.floor(
    (now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24)
  );
  if (diffInDays === 0) return "Today";
  if (diffInDays === 1) return "Yesterday";
  if (diffInDays < 7) return `${diffInDays} days ago`;
  if (diffInDays < 30) return `${Math.floor(diffInDays / 7)} weeks ago`;
  return `${Math.floor(diffInDays / 30)} months ago`;
}

export default function PublicPostDetailPage() {
  const params = useParams();
  const postId = params.id as string;

  const { data: post, isLoading } = useRestateObject<PostResult>(
    "Post",
    postId,
    "get",
    {}
  );

  const [isAdmin, setIsAdmin] = useState(false);

  useEffect(() => {
    isAuthenticated().then(setIsAdmin);
  }, []);

  // Track view
  useEffect(() => {
    if (postId) {
      callObject("Post", postId, "track_view", {}).catch(() => {});
    }
  }, [postId]);

  const handleSourceClick = () => {
    callObject("Post", postId, "track_click", {}).catch(() => {});
  };

  if (isLoading) {
    return (
      <div className="max-w-[1100px] mx-auto px-6 md:px-12 pt-10 pb-20 animate-pulse">
        <div className="h-4 w-24 bg-border-strong rounded mb-8" />
        <div className="bg-surface-raised rounded-lg border border-border p-8">
          <div className="h-8 w-3/4 bg-border rounded mb-4" />
          <div className="h-4 w-1/3 bg-border rounded mb-6" />
          <div className="h-16 w-full bg-surface-muted rounded mb-6" />
          <div className="space-y-3">
            <div className="h-4 w-full bg-border rounded" />
            <div className="h-4 w-5/6 bg-border rounded" />
            <div className="h-4 w-4/6 bg-border rounded" />
          </div>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="max-w-[800px] mx-auto px-6 md:px-12 pt-16 text-center">
        <h1 className="text-xl font-semibold text-text-primary mb-2">
          Post not found
        </h1>
        <BackLink href="/" className="mb-0 justify-center">Back to Home</BackLink>
      </div>
    );
  }

  const tags = post.tags || [];
  const displayTags = tags.filter((t: TagResult) => t.kind !== "post_type");
  const postTypeTag = tags.find((t: TagResult) => t.kind === "post_type");

  const hasDetails = (post.schedules && post.schedules.length > 0) || post.source_url || post.contacts && post.contacts.length > 0;

  return (
    <section className="max-w-[1100px] mx-auto px-6 md:px-12 pt-10 pb-20">
      {/* Back link */}
      <div className="flex items-center justify-between mb-8">
        <BackLink href="/posts" className="mb-0">Back to Resources</BackLink>
        {isAdmin && (
          <Link
            href={`/admin/posts/${postId}`}
            className="inline-flex items-center gap-1.5 text-xs font-medium text-text-muted hover:text-text-primary bg-surface-raised/60 hover:bg-surface-raised border border-border rounded-lg px-3 py-1.5 transition-colors"
          >
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
            </svg>
            Edit
          </Link>
        )}
      </div>

      {/* Two-column: Content + Sidebar */}
      <div className={`grid gap-5 ${hasDetails ? "md:grid-cols-[1fr_280px]" : ""}`}>
        {/* Main content card */}
        <div className="order-1">
          <div className="bg-surface-raised rounded-xl border border-border p-6 sm:p-8 shadow-sm">
            {/* Urgent notes */}
            {post.urgent_notes && post.urgent_notes.length > 0 && (
              <div className="mb-4 px-4 py-3 rounded-lg bg-red-50 border border-red-200">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-red-100 text-red-800">Urgent</span>
                </div>
                <div className="space-y-1.5 mt-2">
                  {post.urgent_notes.map((note, i) => (
                    <div key={i}>
                      {note.cta_text && (
                        <p className="text-sm font-semibold text-red-900">{note.cta_text}</p>
                      )}
                      <p className="text-sm text-red-800 leading-relaxed">{note.content}</p>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Title */}
            <h1 className="text-2xl sm:text-3xl font-bold text-text-primary leading-tight mb-3">
              {post.title}
            </h1>

            {/* Meta */}
            <div className="flex flex-wrap items-center gap-4 text-sm text-text-muted mb-3">
              <span>{formatTimeAgo(post.published_at || post.created_at)}</span>
              {post.location && (
                <span className="inline-flex items-center gap-1.5">
                  <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 11a3 3 0 11-6 0 3 3 0 016 0z" />
                  </svg>
                  {post.location}
                </span>
              )}
            </div>

            {/* Description */}
            <div className="prose max-w-none">
              <ReactMarkdown
                components={{
                  p: ({ children }) => (
                    <p className="mb-4 text-text-body leading-relaxed">{children}</p>
                  ),
                  ul: ({ children }) => (
                    <ul className="list-disc pl-6 mb-4 space-y-1">{children}</ul>
                  ),
                  ol: ({ children }) => (
                    <ol className="list-decimal pl-6 mb-4 space-y-1">{children}</ol>
                  ),
                  li: ({ children }) => (
                    <li className="text-text-body">{children}</li>
                  ),
                  strong: ({ children }) => (
                    <strong className="font-semibold text-text-primary">{children}</strong>
                  ),
                  a: ({ href, children }) => (
                    <a
                      href={href}
                      className="text-link hover:text-link-hover underline"
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      {children}
                    </a>
                  ),
                  h1: ({ children }) => (
                    <h2 className="text-xl font-bold text-text-primary mt-6 mb-3">{children}</h2>
                  ),
                  h2: ({ children }) => (
                    <h3 className="text-lg font-bold text-text-primary mt-5 mb-2">{children}</h3>
                  ),
                  h3: ({ children }) => (
                    <h4 className="text-base font-semibold text-text-primary mt-4 mb-2">{children}</h4>
                  ),
                }}
              >
                {post.description_markdown || post.description || ""}
              </ReactMarkdown>
            </div>

            {/* Tags */}
            {(postTypeTag || displayTags.length > 0) && (
              <div className="flex flex-wrap gap-2 mt-6 pt-6 border-t border-border">
                {postTypeTag && (
                  <span
                    title={`${postTypeTag.kind}: ${postTypeTag.value}`}
                    className={`px-3 py-1 rounded-full text-xs font-medium ${!postTypeTag.color ? "bg-surface-muted text-text-secondary" : ""}`}
                    style={postTypeTag.color ? { backgroundColor: postTypeTag.color + "20", color: postTypeTag.color } : undefined}
                  >
                    {postTypeTag.display_name || formatCategory(postTypeTag.value)}
                  </span>
                )}
                {displayTags.map((tag: TagResult) => (
                  <span
                    key={tag.id}
                    title={`${tag.kind}: ${tag.value}`}
                    className={`px-3 py-1 rounded-full text-xs font-medium ${!tag.color ? "bg-surface-muted text-text-secondary" : ""}`}
                    style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
                  >
                    {tag.display_name || formatCategory(tag.value)}
                  </span>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Sidebar */}
        {hasDetails && (
          <div className="order-2 flex flex-col gap-5">
            {/* Schedule card */}
            {post.schedules && post.schedules.length > 0 && (() => {
              const oneOffSchedules = post.schedules!.filter((s: PostScheduleResult) => !s.rrule);
              const allOneOffsExpired = oneOffSchedules.length > 0 && oneOffSchedules.every(isScheduleExpired);
              return (
                <div className="bg-surface-raised rounded-xl border border-border p-5 shadow-sm">
                  <h3 className="text-xs font-semibold text-text-label uppercase tracking-wider mb-3">Schedule</h3>
                  {allOneOffsExpired && (
                    <div className="mb-3 px-3 py-2 bg-surface-muted border border-border rounded-lg text-xs font-medium text-text-muted">
                      This event has passed
                    </div>
                  )}
                  <div className="space-y-3">
                    {post.schedules!.map((s: PostScheduleResult) => (
                      <div key={s.id} className={`flex items-start gap-2.5 ${isScheduleExpired(s) ? "opacity-60" : ""}`}>
                        <svg className="w-4 h-4 mt-0.5 flex-shrink-0 text-border-strong" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                        <span className="text-sm text-text-body leading-snug">{formatSchedule(s)}</span>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })()}

            {/* Contacts card */}
            {post.contacts && post.contacts.length > 0 && (
              <div className="bg-surface-raised rounded-xl border border-border p-5 shadow-sm">
                <h3 className="text-xs font-semibold text-text-label uppercase tracking-wider mb-3">Contact</h3>
                <div className="space-y-2">
                  {post.contacts.map((c) => (
                    <div key={c.id} className="text-sm text-text-body">
                      {c.contact_label && <span className="text-xs text-text-label block">{c.contact_label}</span>}
                      {c.contact_type === "url" ? (
                        <a href={c.contact_value.startsWith("http") ? c.contact_value : `https://${c.contact_value}`} target="_blank" rel="noopener noreferrer" className="text-link hover:text-link-hover underline break-all">{c.contact_value}</a>
                      ) : c.contact_type === "email" ? (
                        <a href={`mailto:${c.contact_value}`} className="text-link hover:text-link-hover underline">{c.contact_value}</a>
                      ) : c.contact_type === "phone" ? (
                        <a href={`tel:${c.contact_value}`} className="text-link hover:text-link-hover underline">{c.contact_value}</a>
                      ) : (
                        <span>{c.contact_value}</span>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Source link card */}
            {post.source_url && (
              <a
                href={
                  post.source_url.startsWith("http")
                    ? post.source_url
                    : `https://${post.source_url}`
                }
                target="_blank"
                rel="noopener noreferrer"
                onClick={handleSourceClick}
                className="flex items-center justify-center gap-2 bg-action text-surface text-sm font-semibold rounded-xl px-5 py-3.5 hover:bg-action-hover transition-colors shadow-sm"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
                </svg>
                Visit Source
              </a>
            )}
          </div>
        )}

        {/* Comments — same column as main card on desktop, after sidebar on mobile */}
        <div className="order-3 md:col-start-1 md:col-end-2">
          <CommentsSection postId={postId} />
        </div>
      </div>
    </section>
  );
}
