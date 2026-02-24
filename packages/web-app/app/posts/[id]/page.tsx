"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useQuery, useMutation } from "urql";
import { useEffect, useState } from "react";
import { PostDetailPublicQuery, TrackPostViewMutation, TrackPostClickMutation } from "@/lib/graphql/public";
import { isAuthenticated } from "@/lib/auth/actions";
import CommentsSection from "@/components/CommentsSection";

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

interface Schedule {
  id: string;
  dayOfWeek?: number | null;
  opensAt?: string | null;
  closesAt?: string | null;
  timezone: string;
  notes?: string | null;
  rrule?: string | null;
  dtstart?: string | null;
  dtend?: string | null;
  isAllDay: boolean;
  durationMinutes?: number | null;
}

function formatSchedule(s: Schedule): string {
  // One-off event: has dtstart, no day_of_week
  if (s.dtstart && s.dayOfWeek == null) {
    const date = new Date(s.dtstart);
    const dateStr = date.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
    const timeStr = s.opensAt && s.closesAt
      ? `${formatTime12h(s.opensAt)} – ${formatTime12h(s.closesAt)}`
      : s.opensAt ? formatTime12h(s.opensAt) : "";
    const parts = [dateStr, timeStr].filter(Boolean).join("  ");
    return s.notes ? `${parts} (${s.notes})` : parts;
  }

  // Recurring: has day_of_week
  const dayName = s.dayOfWeek != null ? DAY_NAMES[s.dayOfWeek] : "";
  const timeStr = s.opensAt && s.closesAt
    ? `${formatTime12h(s.opensAt)} – ${formatTime12h(s.closesAt)}`
    : s.opensAt ? formatTime12h(s.opensAt) : "";

  let suffix = "";
  if (s.rrule?.includes("INTERVAL=2")) suffix = " (every other week)";
  if (s.rrule?.includes("FREQ=MONTHLY")) suffix = " (monthly)";
  if (s.notes) suffix = ` (${s.notes})`;

  return [dayName, timeStr, suffix].filter(Boolean).join("  ");
}

function isScheduleExpired(s: Schedule): boolean {
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

  const [{ data, fetching: isLoading }] = useQuery({
    query: PostDetailPublicQuery,
    variables: { id: postId },
  });

  const post = data?.post;

  const [, trackView] = useMutation(TrackPostViewMutation);
  const [, trackClick] = useMutation(TrackPostClickMutation);

  const [isAdmin, setIsAdmin] = useState(false);

  useEffect(() => {
    isAuthenticated().then(setIsAdmin);
  }, []);

  // Track view
  useEffect(() => {
    if (postId) {
      trackView({ postId }).catch(() => {});
    }
  }, [postId, trackView]);

  const handleSourceClick = () => {
    trackClick({ postId }).catch(() => {});
  };

  if (isLoading) {
    return (
      <div className="max-w-[1100px] mx-auto px-6 md:px-12 pt-10 pb-20 animate-pulse">
        <div className="h-4 w-24 bg-border-strong mb-8" />
        <div className="bg-surface-raised border border-border p-8">
          <div className="h-8 w-3/4 bg-border mb-4" />
          <div className="h-4 w-1/3 bg-border mb-6" />
          <div className="h-16 w-full bg-surface-muted mb-6" />
          <div className="space-y-3">
            <div className="h-4 w-full bg-border" />
            <div className="h-4 w-5/6 bg-border" />
            <div className="h-4 w-4/6 bg-border" />
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
        <Link href="/" className="text-sm text-text-secondary hover:text-text-primary">
          &larr; Back to Home
        </Link>
      </div>
    );
  }

  const tags = post.tags || [];
  const displayTags = tags.filter((t) => t.kind !== "post_type");
  const postTypeTag = tags.find((t) => t.kind === "post_type");

  const hasDetails = (post.schedules && post.schedules.length > 0) || post.sourceUrl || (post.contacts && post.contacts.length > 0);

  return (
    <section className="max-w-[1100px] mx-auto px-6 md:px-12 pt-10 pb-20">
      {/* Back link */}
      <div className="flex items-center justify-between mb-8">
        <Link href="/posts" className="text-sm text-text-secondary hover:text-text-primary">
          &larr; Back to Resources
        </Link>
        {isAdmin && (
          <Link
            href={`/admin/posts/${postId}`}
            className="text-xs font-medium text-text-muted hover:text-text-primary border border-border px-2 py-1"
          >
            Edit
          </Link>
        )}
      </div>

      {/* Two-column: Content + Sidebar */}
      <div className={`grid gap-5 ${hasDetails ? "md:grid-cols-[1fr_280px]" : ""}`}>
        {/* Main content card */}
        <div className="order-1">
          <div className="bg-surface-raised border border-border p-6 sm:p-8">
            {/* Urgent notes */}
            {post.urgentNotes && post.urgentNotes.length > 0 && (
              <div className="mb-4 px-4 py-3 bg-red-50 border border-red-200">
                <div className="flex items-center gap-2 mb-1">
                  <span className="px-2 py-0.5 text-xs font-medium bg-red-100 text-red-800">Urgent</span>
                </div>
                <div className="space-y-1.5 mt-2">
                  {post.urgentNotes.map((note, i) => (
                    <div key={i}>
                      {note.ctaText && (
                        <p className="text-sm font-semibold text-red-900">{note.ctaText}</p>
                      )}
                      <p className="text-sm text-red-800 leading-relaxed">{note.content}</p>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Organization */}
            {post.organizationName && (
              <p className="text-xs font-medium text-text-muted uppercase tracking-wide mb-1">
                {post.organizationId ? (
                  <Link
                    href={`/organizations/${post.organizationId}`}
                    className="hover:text-text-primary"
                  >
                    {post.organizationName}
                  </Link>
                ) : (
                  post.organizationName
                )}
              </p>
            )}

            {/* Title */}
            <h1 className="text-2xl sm:text-3xl font-bold text-text-primary leading-tight mb-3">
              {post.title}
            </h1>

            {/* Meta */}
            <div className="flex flex-wrap items-center gap-4 text-sm text-text-muted mb-3">
              <span>{formatTimeAgo(post.publishedAt || post.createdAt)}</span>
              {post.location && (
                <span>{post.location}</span>
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
                {post.descriptionMarkdown || post.description || ""}
              </ReactMarkdown>
            </div>

            {/* Tags */}
            {(postTypeTag || displayTags.length > 0) && (
              <div className="flex flex-wrap gap-2 mt-6 pt-6 border-t border-border">
                {postTypeTag && (
                  <span
                    title={`${postTypeTag.kind}: ${postTypeTag.value}`}
                    className={`px-3 py-1 text-xs font-medium ${!postTypeTag.color ? "bg-surface-muted text-text-secondary" : ""}`}
                    style={postTypeTag.color ? { backgroundColor: postTypeTag.color + "20", color: postTypeTag.color } : undefined}
                  >
                    {postTypeTag.displayName || formatCategory(postTypeTag.value)}
                  </span>
                )}
                {displayTags.map((tag) => (
                  <span
                    key={tag.id}
                    title={`${tag.kind}: ${tag.value}`}
                    className={`px-3 py-1 text-xs font-medium ${!tag.color ? "bg-surface-muted text-text-secondary" : ""}`}
                    style={tag.color ? { backgroundColor: tag.color + "20", color: tag.color } : undefined}
                  >
                    {tag.displayName || formatCategory(tag.value)}
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
              const oneOffSchedules = post.schedules!.filter((s) => !s.rrule);
              const allOneOffsExpired = oneOffSchedules.length > 0 && oneOffSchedules.every(isScheduleExpired);
              return (
                <div className="bg-surface-raised border border-border p-5">
                  <h3 className="text-xs font-semibold text-text-label uppercase tracking-wider mb-3">Schedule</h3>
                  {allOneOffsExpired && (
                    <div className="mb-3 px-3 py-2 bg-surface-muted border border-border text-xs font-medium text-text-muted">
                      This event has passed
                    </div>
                  )}
                  <div className="space-y-3">
                    {post.schedules!.map((s) => (
                      <div key={s.id} className={`text-sm text-text-body ${isScheduleExpired(s) ? "opacity-60" : ""}`}>
                        {formatSchedule(s)}
                      </div>
                    ))}
                  </div>
                </div>
              );
            })()}

            {/* Contacts card */}
            {post.contacts && post.contacts.length > 0 && (
              <div className="bg-surface-raised border border-border p-5">
                <h3 className="text-xs font-semibold text-text-label uppercase tracking-wider mb-3">Contact</h3>
                <div className="space-y-2">
                  {post.contacts.map((c) => (
                    <div key={c.id} className="text-sm text-text-body">
                      {c.contactLabel && <span className="text-xs text-text-label block">{c.contactLabel}</span>}
                      {c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social" ? (
                        <a href={c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`} target="_blank" rel="noopener noreferrer" className="text-link hover:text-link-hover underline break-all">{c.contactValue}</a>
                      ) : c.contactType === "email" ? (
                        <a href={`mailto:${c.contactValue}`} className="text-link hover:text-link-hover underline">{c.contactValue}</a>
                      ) : c.contactType === "phone" ? (
                        <a href={`tel:${c.contactValue}`} className="text-link hover:text-link-hover underline">{c.contactValue}</a>
                      ) : (
                        <span>{c.contactValue}</span>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Source link card */}
            {post.sourceUrl && (
              <a
                href={
                  post.sourceUrl.startsWith("http")
                    ? post.sourceUrl
                    : `https://${post.sourceUrl}`
                }
                target="_blank"
                rel="noopener noreferrer"
                onClick={handleSourceClick}
                className="block text-center bg-action text-text-on-action text-sm font-semibold px-4 py-2 hover:bg-action-hover"
              >
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
