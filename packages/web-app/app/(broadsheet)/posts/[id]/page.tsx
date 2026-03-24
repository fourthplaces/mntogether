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
import { PostDetailPublicQuery, TrackPostViewMutation } from "@/lib/graphql/public";
import { isAuthenticated } from "@/lib/auth/actions";
import { resolveDetailVariants } from "@/lib/broadsheet/detail-variants";
import { formatPostDate, formatDeadline as formatDeadlineMN } from "@/lib/broadsheet/dates";

// Broadsheet detail components
import { NewspaperFrame } from "@/components/broadsheet/layout/NewspaperFrame";
import { ArticlePage } from "@/components/broadsheet/detail/ArticlePage";
import { ArticleNav } from "@/components/broadsheet/detail/ArticleNav";
import { TitleA, TitleB } from "@/components/broadsheet/detail/Title";
import { BodyA } from "@/components/broadsheet/detail/BodyA";
import { BodyB } from "@/components/broadsheet/detail/BodyB";
import { KickerA, KickerB } from "@/components/broadsheet/detail/Kicker";
import { ArticleMeta } from "@/components/broadsheet/detail/ArticleMeta";
import { PhotoA } from "@/components/broadsheet/detail/Photo";
import { PhoneA } from "@/components/broadsheet/detail/Phone";
import { EmailA } from "@/components/broadsheet/detail/Email";
import { WebsiteA } from "@/components/broadsheet/detail/Website";
import { AddressA } from "@/components/broadsheet/detail/Address";
import { LinksA } from "@/components/broadsheet/detail/Links";
import { ResourceListA } from "@/components/broadsheet/detail/List";
import { SidebarCard } from "@/components/broadsheet/detail/SidebarCard";
import { HoursScheduleLarge } from "@/components/broadsheet/detail/hours/HoursSchedule";
import { AstRenderer } from "@/components/broadsheet/detail/AstRenderer";
import { RelatedA } from "@/components/broadsheet/detail/Related";
import { PersonCardA } from "@/components/broadsheet/detail/PersonCard";
import { EventDateA } from "@/components/broadsheet/detail/EventDate";
import { StatusBadgeA } from "@/components/broadsheet/detail/StatusBadge";
import { SourceAttributionA } from "@/components/broadsheet/detail/SourceAttribution";
import { UrgentBanner } from "@/components/broadsheet/detail/UrgentBanner";

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
  metaParts.push(formatPostDate(post.publishedAt || post.createdAt));
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

  const hasContacts = contacts.length > 0;

  // ── Full-width header section (above the 2-column grid) ──
  const headerContent = (
    <>
      <ArticleNav />

      {/* Urgent notes banner */}
      {post.urgentNotes && post.urgentNotes.length > 0 && (
        <UrgentBanner notes={post.urgentNotes} />
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
    </>
  );

  // ── Main column (body content, inside 2/3 grid) ──
  const mainContent = (
    <>
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
        <SourceAttributionA
          attribution={sourceAttribution.attribution}
          sourceName={sourceAttribution.sourceName}
        />
      )}

      {/* Related posts */}
      {post.relatedPosts && post.relatedPosts.length > 0 && (
        <RelatedA
          articles={post.relatedPosts.map((rp) => {
            const ptTag = rp.tags?.find((t) => t.kind === "post_type");
            return {
              id: rp.id,
              kicker: ptTag?.displayName || formatCategory(rp.postType || "post"),
              title: rp.title,
              color: ptTag?.color || undefined,
            };
          })}
        />
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

      {/* Datetime — event dates */}
      {post.datetime && post.datetime.start && (
        <SidebarCard header="Event">
          <EventDateA
            start={post.datetime.start}
            end={post.datetime.end}
            cost={post.datetime.cost}
          />
        </SidebarCard>
      )}

      {/* Contact info */}
      {hasContacts && (
        <SidebarCard header="Contact">
          <div className="sidebar-card__contacts">
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
              <EmailA
                key={c.id}
                email={{
                  address: c.contactValue,
                  label: c.contactLabel || undefined,
                }}
              />
            ))}
            {websites.map((c) => (
              <WebsiteA
                key={c.id}
                website={{
                  url: c.contactValue.startsWith("http") ? c.contactValue : `https://${c.contactValue}`,
                  label: c.contactLabel || undefined,
                }}
              />
            ))}
          </div>
        </SidebarCard>
      )}

      {/* Link from field groups */}
      {post.link && post.link.url && (
        <SidebarCard header="Link">
          <LinksA
            links={[
              {
                title: post.link.label || post.link.url.replace(/^https?:\/\//, ""),
                url: post.link.url.startsWith("http") ? post.link.url : `https://${post.link.url}`,
              },
            ]}
            header=""
          />
          {post.link.deadline && (
            <div className="link-deadline">
              Deadline: {formatDeadlineMN(post.link.deadline)}
            </div>
          )}
        </SidebarCard>
      )}

      {/* Items list (resources, inventory, etc.) */}
      {post.items && post.items.length > 0 && (
        <SidebarCard header="Items">
          <ResourceListA items={post.items.map((item) => ({ name: item.name, detail: item.detail || "" }))} />
        </SidebarCard>
      )}

      {/* Person (spotlight, about) */}
      {post.person && post.person.name && (
        <SidebarCard header="About">
          <PersonCardA person={{
            name: post.person.name,
            role: post.person.role,
            bio: post.person.bio,
            photoUrl: post.person.photoUrl,
            quote: post.person.quote,
          }} />
        </SidebarCard>
      )}

      {/* Source link CTA */}
      {post.sourceUrl && (
        <SidebarCard header="Source">
          <LinksA
            links={[
              {
                title: sourceAttribution?.sourceName || "Original Source",
                url: post.sourceUrl.startsWith("http") ? post.sourceUrl : `https://${post.sourceUrl}`,
              },
            ]}
            header=""
          />
        </SidebarCard>
      )}

      {/* Post status badge */}
      {post.postStatus && post.postStatus.state && (
        <StatusBadgeA
          state={post.postStatus.state}
          verified={post.postStatus.verified}
        />
      )}
    </>
  );

  return (
    <>
      {isAdmin && (
        <div className="admin-bar">
          <div className="admin-bar__inner">
            <span className="admin-bar__badge">Admin</span>
            <span>Viewing published post</span>
            <Link
              href={`/admin/posts/${postId}`}
              className="admin-bar__link"
            >
              Edit in CMS &rarr;
            </Link>
          </div>
        </div>
      )}
      <NewspaperFrame>
        {headerContent}
        <ArticlePage main={mainContent} sidebar={sidebarContent} />
      </NewspaperFrame>
    </>
  );
}
