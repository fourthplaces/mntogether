"use client";

/**
 * Post detail page — broadsheet article layout.
 *
 * Uses the prototype detail component library (ArticlePage, TitleA/B, BodyA/B,
 * KickerA/B, ArticleMeta, Photo, SidebarCard, etc.) with broadsheet-detail.css.
 */

import Link from "next/link";
import { useParams } from "next/navigation";
import ReactMarkdown from "react-markdown";
import { useQuery, useMutation } from "urql";
import { useEffect, useState } from "react";
import { PostDetailPublicQuery, TrackPostViewMutation, TrackPostClickMutation } from "@/lib/graphql/public";
import { isAuthenticated } from "@/lib/auth/actions";
import { resolveDetailVariants } from "@/lib/broadsheet/detail-variants";

// Broadsheet detail components
import { ArticlePage } from "@/components/broadsheet/detail/ArticlePage";
import { ArticleNav } from "@/components/broadsheet/detail/ArticleNav";
import { TitleA, TitleB } from "@/components/broadsheet/detail/Title";
import { BodyA } from "@/components/broadsheet/detail/BodyA";
import { BodyB } from "@/components/broadsheet/detail/BodyB";
import { KickerA, KickerB } from "@/components/broadsheet/detail/Kicker";
import { ArticleMeta } from "@/components/broadsheet/detail/ArticleMeta";
import { PhotoA } from "@/components/broadsheet/detail/Photo";
import { PhoneA } from "@/components/broadsheet/detail/Phone";
import { AddressA } from "@/components/broadsheet/detail/Address";
import { LinksA } from "@/components/broadsheet/detail/Links";
import { SidebarCard } from "@/components/broadsheet/detail/SidebarCard";
import { HoursScheduleLarge } from "@/components/broadsheet/detail/hours/HoursSchedule";
import { AstRenderer } from "@/components/broadsheet/detail/AstRenderer";

import type { WeekSchedule } from "@/lib/broadsheet/hours";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatCategory(value: string): string {
  return value
    .split("-")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
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

const DAY_NAMES_FULL = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

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
  if (s.dtstart && s.dayOfWeek == null) {
    const date = new Date(s.dtstart);
    const dateStr = date.toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
    const timeStr = s.opensAt && s.closesAt
      ? `${formatTime12h(s.opensAt)} \u2013 ${formatTime12h(s.closesAt)}`
      : s.opensAt ? formatTime12h(s.opensAt) : "";
    const parts = [dateStr, timeStr].filter(Boolean).join("  ");
    return s.notes ? `${parts} (${s.notes})` : parts;
  }
  const dayName = s.dayOfWeek != null ? DAY_NAMES_FULL[s.dayOfWeek] : "";
  const timeStr = s.opensAt && s.closesAt
    ? `${formatTime12h(s.opensAt)} \u2013 ${formatTime12h(s.closesAt)}`
    : s.opensAt ? formatTime12h(s.opensAt) : "";
  let suffix = "";
  if (s.rrule?.includes("INTERVAL=2")) suffix = " (every other week)";
  if (s.rrule?.includes("FREQ=MONTHLY")) suffix = " (monthly)";
  if (s.notes) suffix = ` (${s.notes})`;
  return [dayName, timeStr, suffix].filter(Boolean).join("  ");
}

/**
 * Convert BroadsheetScheduleEntry[] from field groups into a WeekSchedule
 * for the HoursSchedule component.
 */
function toWeekSchedule(entries: Array<{ day: string; opens: string; closes: string }>): WeekSchedule | null {
  if (!entries || entries.length === 0) return null;

  const dayMap: Record<string, number> = {
    sunday: 0, monday: 1, tuesday: 2, wednesday: 3,
    thursday: 4, friday: 5, saturday: 6,
    sun: 0, mon: 1, tue: 2, wed: 3, thu: 4, fri: 5, sat: 6,
  };

  const week: WeekSchedule = [null, null, null, null, null, null, null];
  for (const entry of entries) {
    const dayIndex = dayMap[entry.day.toLowerCase()];
    if (dayIndex != null) {
      week[dayIndex] = { opens: entry.opens.slice(0, 5), closes: entry.closes.slice(0, 5) };
    }
  }

  // Only return if at least one day was set
  return week.some(d => d !== null) ? week : null;
}

// ---------------------------------------------------------------------------
// Page component
// ---------------------------------------------------------------------------

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

  // Loading skeleton
  if (isLoading) {
    return (
      <div className="article-page" style={{ padding: "2rem", maxWidth: "72rem", margin: "0 auto" }}>
        <div style={{ marginBottom: "2rem" }}>
          <div className="skeleton-line" style={{ height: "0.75rem", width: "8rem", marginBottom: "1.5rem" }} />
          <div className="skeleton-line" style={{ height: "2.5rem", width: "66%", marginBottom: "0.75rem" }} />
          <div className="skeleton-line" style={{ height: "1rem", width: "40%", marginBottom: "2rem" }} />
          <div className="skeleton-line" style={{ height: "16rem", width: "100%", marginBottom: "1.5rem" }} />
          <div className="skeleton-line" style={{ height: "1rem", width: "100%", marginBottom: "0.5rem" }} />
          <div className="skeleton-line" style={{ height: "1rem", width: "90%", marginBottom: "0.5rem" }} />
          <div className="skeleton-line" style={{ height: "1rem", width: "75%" }} />
        </div>
      </div>
    );
  }

  // Not found
  if (!post) {
    return (
      <div style={{ textAlign: "center", padding: "4rem 2rem" }}>
        <h1 style={{ fontFamily: "var(--font-display)", marginBottom: "1rem" }}>Post not found</h1>
        <Link href="/" style={{ color: "var(--sienna, #a0522d)" }}>
          &larr; Back to front page
        </Link>
      </div>
    );
  }

  // Resolve component variants based on post type
  const variants = resolveDetailVariants(post.postType);

  // Extract data for components
  const tags = post.tags || [];
  const displayTags = tags.filter((t) => t.kind !== "post_type");
  const postTypeTag = tags.find((t) => t.kind === "post_type");
  const categoryTags = displayTags.map((t) => t.displayName || formatCategory(t.value));

  // Build article meta parts (byline · date · location)
  const metaParts: string[] = [];
  if (post.meta?.byline) metaParts.push(post.meta.byline);
  metaParts.push(formatTimeAgo(post.publishedAt || post.createdAt));
  if (post.location) metaParts.push(post.location);

  // Hero photo from media field group
  const heroMedia = post.media && post.media.length > 0 ? post.media[0] : null;

  // Source attribution
  const sourceAttribution = post.sourceAttribution;

  // Schedule from field groups (for HoursSchedule widget)
  const weekSchedule = post.schedule ? toWeekSchedule(post.schedule) : null;

  // Contacts from the traditional contacts field
  const contacts = post.contacts || [];
  const phones = contacts.filter(c => c.contactType === "phone");
  const emails = contacts.filter(c => c.contactType === "email");
  const websites = contacts.filter(c => c.contactType === "website" || c.contactType === "booking_url" || c.contactType === "social");

  // Check if sidebar has content
  const hasSchedules = (post.schedules && post.schedules.length > 0) || weekSchedule;
  const hasContacts = contacts.length > 0;
  const hasSource = !!post.sourceUrl || !!sourceAttribution;
  const hasSidebar = hasSchedules || hasContacts || hasSource;

  // ── Main content ──
  const mainContent = (
    <>
      <ArticleNav />

      {/* Admin edit link */}
      {isAdmin && (
        <div style={{ marginBottom: "0.5rem" }}>
          <Link
            href={`/admin/posts/${postId}`}
            style={{
              fontSize: "0.75rem",
              fontFamily: "var(--font-mono)",
              color: "var(--sienna, #a0522d)",
              textDecoration: "none",
              opacity: 0.7,
            }}
          >
            Edit in CMS &rarr;
          </Link>
        </div>
      )}

      {/* Urgent notes banner */}
      {post.urgentNotes && post.urgentNotes.length > 0 && (
        <div className="callout" style={{ borderColor: "var(--burnt-clay, #c0392b)", marginBottom: "1.5rem" }}>
          <div style={{ fontFamily: "var(--font-condensed)", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.05em", marginBottom: "0.25rem" }}>
            Urgent
          </div>
          {post.urgentNotes.map((note, i) => (
            <div key={i}>
              {note.ctaText && <p style={{ fontWeight: 600 }}>{note.ctaText}</p>}
              <p>{note.content}</p>
            </div>
          ))}
        </div>
      )}

      {/* Kicker (category tags) */}
      {categoryTags.length > 0 && (
        variants.kickerVariant === "B" && categoryTags.length > 0 ? (
          <KickerB
            primary={categoryTags[0]}
            secondary={categoryTags.slice(1)}
          />
        ) : (
          <KickerA tags={[
            ...(postTypeTag ? [postTypeTag.displayName || formatCategory(postTypeTag.value)] : []),
            ...categoryTags,
          ]} />
        )
      )}

      {/* Title */}
      {variants.titleVariant === "B" ? (
        <TitleB size={variants.titleSize}>
          {post.title}
        </TitleB>
      ) : (
        <TitleA size={variants.titleSize} deck={post.meta?.deck || undefined}>
          {post.title}
        </TitleA>
      )}

      {/* Article meta (byline · date · location) */}
      {metaParts.length > 0 && <ArticleMeta parts={metaParts} />}

      {/* Hero photo */}
      {heroMedia && heroMedia.imageUrl && (
        <PhotoA
          photo={{
            src: heroMedia.imageUrl,
            alt: heroMedia.caption || "",
            caption: heroMedia.caption || "",
            credit: heroMedia.credit || "",
          }}
        />
      )}

      {/* Body content — render from AST if available, fall back to markdown */}
      {post.bodyAst ? (
        (() => {
          try {
            const ast = JSON.parse(post.bodyAst);
            const BodyWrapper = variants.bodyVariant === "B" ? BodyB : BodyA;
            return (
              <BodyWrapper>
                <AstRenderer value={ast} className="" />
              </BodyWrapper>
            );
          } catch {
            return null;
          }
        })()
      ) : post.bodyRaw ? (
        variants.bodyVariant === "B" ? (
          <BodyB>
            <ReactMarkdown
              components={{
                a: ({ href, children }) => (
                  <a href={href} target="_blank" rel="noopener noreferrer">{children}</a>
                ),
                h1: ({ children }) => <h2>{children}</h2>,
                h2: ({ children }) => <h3>{children}</h3>,
                h3: ({ children }) => <h4>{children}</h4>,
              }}
            >
              {post.bodyRaw}
            </ReactMarkdown>
          </BodyB>
        ) : (
          <BodyA>
            <ReactMarkdown
              components={{
                a: ({ href, children }) => (
                  <a href={href} target="_blank" rel="noopener noreferrer">{children}</a>
                ),
                h1: ({ children }) => <h2>{children}</h2>,
                h2: ({ children }) => <h3>{children}</h3>,
                h3: ({ children }) => <h4>{children}</h4>,
              }}
            >
              {post.bodyRaw}
            </ReactMarkdown>
          </BodyA>
        )
      ) : null}

      {/* Source attribution at bottom of article */}
      {sourceAttribution && (sourceAttribution.sourceName || sourceAttribution.attribution) && (
        <div className="article-meta mono-sm" style={{ marginTop: "2rem", opacity: 0.7 }}>
          {sourceAttribution.attribution && <span>{sourceAttribution.attribution}</span>}
          {sourceAttribution.sourceName && sourceAttribution.attribution && <span> · </span>}
          {sourceAttribution.sourceName && <span>{sourceAttribution.sourceName}</span>}
        </div>
      )}
    </>
  );

  // ── Sidebar content ──
  const sidebarContent = (
    <>
      {/* Hours schedule from field groups */}
      {weekSchedule && (
        <SidebarCard header="Hours">
          <HoursScheduleLarge week={weekSchedule} />
        </SidebarCard>
      )}

      {/* Legacy schedules (events, one-off times) */}
      {post.schedules && post.schedules.length > 0 && !weekSchedule && (
        <SidebarCard header="Schedule">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.375rem" }}>
            {post.schedules.map((s) => (
              <div key={s.id} className="mono-sm">
                {formatSchedule(s)}
              </div>
            ))}
          </div>
        </SidebarCard>
      )}

      {/* Datetime from field groups (event dates) */}
      {post.datetime && post.datetime.start && (
        <SidebarCard header="Event">
          <div className="mono-sm">
            <div>{new Date(post.datetime.start).toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric", year: "numeric" })}</div>
            {post.datetime.end && (
              <div style={{ opacity: 0.7 }}>
                through {new Date(post.datetime.end).toLocaleDateString("en-US", { month: "long", day: "numeric" })}
              </div>
            )}
            {post.datetime.cost && (
              <div style={{ marginTop: "0.25rem" }}>{post.datetime.cost}</div>
            )}
          </div>
        </SidebarCard>
      )}

      {/* Contact info */}
      {hasContacts && (
        <SidebarCard header="Contact">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {phones.map((c) => (
              <PhoneA
                key={c.id}
                phone={{
                  number: c.contactValue,
                  display: c.contactValue,
                  label: c.contactLabel || undefined,
                }}
              />
            ))}
            {emails.map((c) => (
              <div key={c.id} className="mono-sm">
                {c.contactLabel && <div style={{ fontWeight: 600, marginBottom: "0.125rem" }}>{c.contactLabel}</div>}
                <a href={`mailto:${c.contactValue}`} style={{ color: "var(--sienna, #a0522d)" }}>{c.contactValue}</a>
              </div>
            ))}
            {websites.map((c) => (
              <div key={c.id} className="mono-sm">
                {c.contactLabel && <div style={{ fontWeight: 600, marginBottom: "0.125rem" }}>{c.contactLabel}</div>}
                <a
                  href={c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: "var(--sienna, #a0522d)", wordBreak: "break-all" }}
                >
                  {c.contactValue.replace(/^https?:\/\//, "")}
                </a>
              </div>
            ))}
          </div>
        </SidebarCard>
      )}

      {/* Address from person/link field groups */}
      {post.link && post.link.url && (
        <SidebarCard header="Link">
          <div className="mono-sm">
            <a
              href={post.link.url.startsWith("http") ? post.link.url : `https://${post.link.url}`}
              target="_blank"
              rel="noopener noreferrer"
              style={{ color: "var(--sienna, #a0522d)" }}
            >
              {post.link.label || post.link.url.replace(/^https?:\/\//, "")}
            </a>
            {post.link.deadline && (
              <div style={{ marginTop: "0.25rem", opacity: 0.7 }}>Deadline: {post.link.deadline}</div>
            )}
          </div>
        </SidebarCard>
      )}

      {/* Items list (resources, inventory, etc.) */}
      {post.items && post.items.length > 0 && (
        <SidebarCard header="Items">
          <div style={{ display: "flex", flexDirection: "column", gap: "0.25rem" }}>
            {post.items.map((item, i) => (
              <div key={i} className="mono-sm">
                <span style={{ fontWeight: 600 }}>{item.name}</span>
                {item.detail && <span style={{ opacity: 0.7 }}> · {item.detail}</span>}
              </div>
            ))}
          </div>
        </SidebarCard>
      )}

      {/* Source link CTA */}
      {post.sourceUrl && (
        <a
          href={post.sourceUrl.startsWith("http") ? post.sourceUrl : `https://${post.sourceUrl}`}
          target="_blank"
          rel="noopener noreferrer"
          onClick={handleSourceClick}
          className="sidebar-card"
          style={{
            display: "block",
            textAlign: "center",
            fontFamily: "var(--font-condensed)",
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.05em",
            color: "var(--sienna, #a0522d)",
            textDecoration: "none",
            padding: "0.75rem 1rem",
          }}
        >
          Visit Source &rarr;
        </a>
      )}

      {/* Post status badge */}
      {post.postStatus && post.postStatus.state && (
        <div className="mono-sm" style={{ opacity: 0.6, marginTop: "0.5rem" }}>
          {post.postStatus.verified && <span>Verified · </span>}
          <span>{post.postStatus.state}</span>
        </div>
      )}
    </>
  );

  // If no sidebar content, render main-only layout
  if (!hasSidebar) {
    return (
      <div className="article-page" style={{ display: "block" }}>
        <div className="article-main">{mainContent}</div>
      </div>
    );
  }

  return <ArticlePage main={mainContent} sidebar={sidebarContent} />;
}
