"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useQuery, useMutation } from "urql";
import { AstRenderer } from "@/components/broadsheet/detail/AstRenderer";
import { useEffect, useState } from "react";
import { PostDetailPublicQuery, TrackPostViewMutation, TrackPostClickMutation } from "@/lib/graphql/public";
import { isAuthenticated } from "@/lib/auth/actions";

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
      <div className="page-section page-section--content skeleton">
        <div className="skeleton-line skeleton-line--strong" style={{ height: "1rem", width: "6rem", marginBottom: "2rem" }} />
        <div className="card" style={{ padding: "2rem" }}>
          <div className="skeleton-line" style={{ height: "2rem", width: "75%", marginBottom: "1rem" }} />
          <div className="skeleton-line" style={{ height: "1rem", width: "33%", marginBottom: "1.5rem" }} />
          <div className="skeleton-line" style={{ height: "4rem", width: "100%", marginBottom: "1.5rem" }} />
          <div className="schedule-list">
            <div className="skeleton-line" style={{ height: "1rem", width: "100%" }} />
            <div className="skeleton-line" style={{ height: "1rem", width: "83%" }} />
            <div className="skeleton-line" style={{ height: "1rem", width: "66%" }} />
          </div>
        </div>
      </div>
    );
  }

  if (!post) {
    return (
      <div className="page-section page-section--narrow page-section--centered">
        <h1 className="card-title--semi" style={{ marginBottom: "0.5rem" }}>
          Post not found
        </h1>
        <Link href="/" className="post-detail-back-link">
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
    <section className="page-section page-section--content">
      {/* Back link */}
      <div className="post-detail-back-row">
        <Link href="/posts" className="post-detail-back-link">
          &larr; Back to Resources
        </Link>
        {isAdmin && (
          <Link
            href={`/admin/posts/${postId}`}
            className="btn-edit"
          >
            Edit
          </Link>
        )}
      </div>

      {/* Two-column: Content + Sidebar */}
      <div className={`post-detail-grid ${hasDetails ? "post-detail-grid--sidebar" : ""}`}>
        {/* Main content card */}
        <div className="post-detail-main">
          <div className="card card--padded">
            {/* Urgent notes */}
            {post.urgentNotes && post.urgentNotes.length > 0 && (
              <div className="urgent-section">
                <div className="urgent-section-header">
                  <span className="badge-urgent">Urgent</span>
                </div>
                <div className="urgent-notes">
                  {post.urgentNotes.map((note, i) => (
                    <div key={i}>
                      {note.ctaText && (
                        <p className="urgent-cta">{note.ctaText}</p>
                      )}
                      <p className="urgent-text">{note.content}</p>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Organization */}
            {post.organizationName && (
              <p className="org-label" style={{ marginBottom: "0.25rem" }}>
                {post.organizationId ? (
                  <Link href={`/organizations/${post.organizationId}`}>
                    {post.organizationName}
                  </Link>
                ) : (
                  post.organizationName
                )}
              </p>
            )}

            {/* Title */}
            <h1 className="post-title">
              {post.title}
            </h1>

            {/* Meta */}
            <div className="post-meta">
              <span>{formatTimeAgo(post.publishedAt || post.createdAt)}</span>
              {post.location && (
                <span>{post.location}</span>
              )}
            </div>

            {/* Description — render from AST if available, fall back to markdown */}
            {post.bodyAst ? (
              (() => {
                try {
                  const ast = JSON.parse(post.bodyAst);
                  return <AstRenderer value={ast} className="body-a" />;
                } catch {
                  return null;
                }
              })()
            ) : (
              <div className="prose">
                <ReactMarkdown
                  components={{
                    a: ({ href, children }) => (
                      <a
                        href={href}
                        target="_blank"
                        rel="noopener noreferrer"
                      >
                        {children}
                      </a>
                    ),
                    h1: ({ children }) => (
                      <h2>{children}</h2>
                    ),
                    h2: ({ children }) => (
                      <h3>{children}</h3>
                    ),
                    h3: ({ children }) => (
                      <h4>{children}</h4>
                    ),
                  }}
                >
                  {post.descriptionMarkdown || post.description || ""}
                </ReactMarkdown>
              </div>
            )}

            {/* Tags */}
            {(postTypeTag || displayTags.length > 0) && (
              <div className="post-tags">
                {postTypeTag && (
                  <span
                    title={`${postTypeTag.kind}: ${postTypeTag.value}`}
                    className={`tag ${!postTypeTag.color ? "tag--muted" : ""}`}
                    style={postTypeTag.color ? { backgroundColor: postTypeTag.color + "20", color: postTypeTag.color } : undefined}
                  >
                    {postTypeTag.displayName || formatCategory(postTypeTag.value)}
                  </span>
                )}
                {displayTags.map((tag) => (
                  <span
                    key={tag.id}
                    title={`${tag.kind}: ${tag.value}`}
                    className={`tag ${!tag.color ? "tag--muted" : ""}`}
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
          <div className="post-detail-sidebar">
            {/* Schedule card */}
            {post.schedules && post.schedules.length > 0 && (() => {
              const oneOffSchedules = post.schedules!.filter((s) => !s.rrule);
              const allOneOffsExpired = oneOffSchedules.length > 0 && oneOffSchedules.every(isScheduleExpired);
              return (
                <div className="card--sidebar">
                  <h3 className="sidebar-heading">Schedule</h3>
                  {allOneOffsExpired && (
                    <div className="notice-box">
                      This event has passed
                    </div>
                  )}
                  <div className="schedule-list">
                    {post.schedules!.map((s) => (
                      <div key={s.id} className={`schedule-item ${isScheduleExpired(s) ? "schedule-item--expired" : ""}`}>
                        {formatSchedule(s)}
                      </div>
                    ))}
                  </div>
                </div>
              );
            })()}

            {/* Contacts card */}
            {post.contacts && post.contacts.length > 0 && (
              <div className="card--sidebar">
                <h3 className="sidebar-heading">Contact</h3>
                <div className="contact-list">
                  {post.contacts.map((c) => (
                    <div key={c.id} className="contact-item">
                      {c.contactLabel && <span className="contact-item-label">{c.contactLabel}</span>}
                      {c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social" ? (
                        <a href={c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`} target="_blank" rel="noopener noreferrer" className="body-link" style={{ wordBreak: "break-all" }}>{c.contactValue}</a>
                      ) : c.contactType === "email" ? (
                        <a href={`mailto:${c.contactValue}`} className="body-link">{c.contactValue}</a>
                      ) : c.contactType === "phone" ? (
                        <a href={`tel:${c.contactValue}`} className="body-link">{c.contactValue}</a>
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
                className="btn-primary--block"
              >
                Visit Source
              </a>
            )}
          </div>
        )}

      </div>
    </section>
  );
}
