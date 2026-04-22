"use client";

/**
 * PostDetailView
 *
 * Shared renderer used by both:
 *   - /posts/[id]               — public article, any viewer
 *   - /preview/posts/[id]       — admin-only draft preview
 *
 * Takes a fully-hydrated post (from either the `post` or `postPreview`
 * query) plus an optional banner slot for a per-context header (e.g. the
 * admin-viewing-published bar, or the PREVIEW banner).
 *
 * Extracting this keeps the two page routes honest-to-goodness thin
 * wrappers around a single render pipeline — no accidental drift
 * between what editors preview and what readers see.
 */

import type { ReactNode } from "react";
import Link from "next/link";
import ReactMarkdown from "react-markdown";

import { resolveDetailVariants } from "@/lib/broadsheet/detail-variants";
import { formatPostDate, formatDeadline as formatDeadlineMN } from "@/lib/broadsheet/dates";

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
 * Section heading for the items list. Returns null when no meaningful
 * label fits — "Items" as a header is noise; the list alone is clear
 * enough in those cases.
 */
function itemsHeadingFor(postType?: string | null): string | null {
  switch (postType) {
    case "need":
      return "What's needed";
    case "aid":
      return "What's available";
    case "reference":
      return "Resources";
    default:
      return null;
  }
}

function toWeekSchedule(
  entries: Array<{ day: string; opens: string; closes: string }>
): WeekSchedule | null {
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
      week[dayIndex] = {
        opens: entry.opens.slice(0, 5),
        closes: entry.closes.slice(0, 5),
      };
    }
  }

  return week.some((d) => d !== null) ? week : null;
}

// ---------------------------------------------------------------------------
// Loading skeleton — exported for use by thin page wrappers
// ---------------------------------------------------------------------------

export function PostDetailSkeleton() {
  return (
    <div
      className="article-page"
      style={{ padding: "2rem", maxWidth: "72rem", margin: "0 auto" }}
    >
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

export function PostNotFound() {
  return (
    <div style={{ textAlign: "center", padding: "4rem 2rem" }}>
      <h1 style={{ fontFamily: "var(--font-display)", marginBottom: "1rem" }}>Post not found</h1>
      <Link href="/" style={{ color: "var(--sienna, #a0522d)" }}>
        &larr; Back to front page
      </Link>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main renderer
// ---------------------------------------------------------------------------

/**
 * Intentionally permissive type — matches both PostDetailPublicQuery and
 * PostPreviewQuery results. All of the optional field groups are either
 * present on both, or the renderer gracefully skips them.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type PostDetailPost = any;

interface Props {
  post: PostDetailPost;
  /** Optional banner rendered above the newspaper frame — used for
   *  "Admin viewing published post" on the public route and
   *  "PREVIEW — Not Published" on the preview route. */
  banner?: ReactNode;
}

export function PostDetailView({ post, banner }: Props) {
  const variants = resolveDetailVariants(post.postType);

  const tags = post.tags || [];
  const displayTags = tags.filter((t: { kind: string }) => t.kind !== "post_type");
  const postTypeTag = tags.find((t: { kind: string }) => t.kind === "post_type");
  const categoryTags = displayTags.map(
    (t: { displayName?: string | null; value: string }) =>
      t.displayName || formatCategory(t.value)
  );

  const metaParts: string[] = [];
  if (post.meta?.byline) metaParts.push(post.meta.byline);
  metaParts.push(formatPostDate(post.publishedAt || post.createdAt));
  if (post.location) metaParts.push(post.location);

  const heroMedia = post.media && post.media.length > 0 ? post.media[0] : null;
  const sourceAttribution = post.sourceAttribution;
  const weekSchedule = post.schedule ? toWeekSchedule(post.schedule) : null;

  const contacts = post.contacts || [];
  const phones = contacts.filter(
    (c: { contactType: string }) => c.contactType === "phone"
  );
  const emails = contacts.filter(
    (c: { contactType: string }) => c.contactType === "email"
  );
  const websites = contacts.filter(
    (c: { contactType: string }) =>
      c.contactType === "website" ||
      c.contactType === "booking_url" ||
      c.contactType === "social"
  );
  const hasContacts = contacts.length > 0;

  // ── Header content (runs above the 2-column grid) ───────────────
  const headerContent = (
    <>
      <ArticleNav />

      {post.urgentNotes && post.urgentNotes.length > 0 && (
        <UrgentBanner notes={post.urgentNotes} />
      )}

      {categoryTags.length > 0 &&
        (variants.kickerVariant === "B" && categoryTags.length > 0 ? (
          <KickerB primary={categoryTags[0]} secondary={categoryTags.slice(1)} />
        ) : (
          <KickerA
            tags={[
              ...(postTypeTag
                ? [postTypeTag.displayName || formatCategory(postTypeTag.value)]
                : []),
              ...categoryTags,
            ]}
          />
        ))}

      {variants.titleVariant === "B" ? (
        <TitleB size={variants.titleSize}>{post.title}</TitleB>
      ) : (
        <TitleA size={variants.titleSize} deck={post.meta?.deck || undefined}>
          {post.title}
        </TitleA>
      )}

      {metaParts.length > 0 && <ArticleMeta parts={metaParts} />}
    </>
  );

  // ── Main column (body inside 2/3 grid) ──────────────────────────
  const mainContent = (
    <>
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

      {post.bodyAst
        ? (() => {
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
        : post.bodyRaw
          ? variants.bodyVariant === "B"
            ? (
              <BodyB>
                <ReactMarkdown
                  components={{
                    a: ({ href, children }) => (
                      <a href={href} target="_blank" rel="noopener noreferrer">
                        {children}
                      </a>
                    ),
                    h1: ({ children }) => <h2>{children}</h2>,
                    h2: ({ children }) => <h3>{children}</h3>,
                    h3: ({ children }) => <h4>{children}</h4>,
                  }}
                >
                  {post.bodyRaw}
                </ReactMarkdown>
              </BodyB>
            )
            : (
              <BodyA>
                <ReactMarkdown
                  components={{
                    a: ({ href, children }) => (
                      <a href={href} target="_blank" rel="noopener noreferrer">
                        {children}
                      </a>
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
          : null}

      {/* Items list — belongs in the main column, not the sidebar. A
       * reference directory, a needs list, or an offers inventory IS
       * the substance of the post, not a secondary aside. The heading
       * adapts to post_type so it reads naturally for each variant. */}
      {post.items && post.items.length > 0 && (() => {
        const heading = itemsHeadingFor(post.type);
        return (
          <section className="article-items">
            {heading && <h2 className="article-items__heading">{heading}</h2>}
            <ResourceListA
              items={post.items.map(
                (item: { name: string; detail?: string | null }) => ({
                  name: item.name,
                  detail: item.detail || "",
                })
              )}
            />
          </section>
        );
      })()}

      {sourceAttribution &&
        (sourceAttribution.sourceName || sourceAttribution.attribution) && (
          <SourceAttributionA
            attribution={sourceAttribution.attribution}
            sourceName={sourceAttribution.sourceName}
          />
        )}

      {post.relatedPosts && post.relatedPosts.length > 0 && (
        <RelatedA
          articles={post.relatedPosts.map(
            (rp: {
              id: string;
              title: string;
              postType?: string | null;
              tags?: Array<{ kind: string; displayName?: string | null; value: string; color?: string | null }>;
            }) => {
              const ptTag = rp.tags?.find((t) => t.kind === "post_type");
              return {
                id: rp.id,
                kicker: ptTag?.displayName || formatCategory(rp.postType || "post"),
                title: rp.title,
                color: ptTag?.color || undefined,
              };
            }
          )}
        />
      )}
    </>
  );

  // ── Sidebar ─────────────────────────────────────────────────────
  const sidebarContent = (
    <>
      {weekSchedule && (
        <SidebarCard header="Hours">
          <HoursScheduleLarge week={weekSchedule} />
        </SidebarCard>
      )}

      {post.datetime && post.datetime.start && (
        <SidebarCard header="Event">
          <EventDateA
            start={post.datetime.start}
            end={post.datetime.end}
            cost={post.datetime.cost}
          />
        </SidebarCard>
      )}

      {hasContacts && (
        <SidebarCard header="Contact">
          <div className="sidebar-card__contacts">
            {phones.map(
              (c: {
                id: string;
                contactValue: string;
                contactLabel?: string | null;
              }) => (
                <PhoneA
                  key={c.id}
                  phone={{
                    number: c.contactValue,
                    display: c.contactValue,
                    label: c.contactLabel || undefined,
                  }}
                />
              )
            )}
            {emails.map(
              (c: {
                id: string;
                contactValue: string;
                contactLabel?: string | null;
              }) => (
                <EmailA
                  key={c.id}
                  email={{
                    address: c.contactValue,
                    label: c.contactLabel || undefined,
                  }}
                />
              )
            )}
            {websites.map(
              (c: {
                id: string;
                contactValue: string;
                contactLabel?: string | null;
              }) => (
                <WebsiteA
                  key={c.id}
                  website={{
                    url: c.contactValue.startsWith("http")
                      ? c.contactValue
                      : `https://${c.contactValue}`,
                    label: c.contactLabel || undefined,
                  }}
                />
              )
            )}
          </div>
        </SidebarCard>
      )}

      {post.link && post.link.url && (
        <SidebarCard header="Link">
          <LinksA
            links={[
              {
                title:
                  post.link.label || post.link.url.replace(/^https?:\/\//, ""),
                url: post.link.url.startsWith("http")
                  ? post.link.url
                  : `https://${post.link.url}`,
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

      {/* Items rendered in the main column — see mainContent above. */}

      {post.person && post.person.name && (
        <SidebarCard header="About">
          <PersonCardA
            person={{
              name: post.person.name,
              role: post.person.role,
              bio: post.person.bio,
              photoUrl: post.person.photoUrl,
              quote: post.person.quote,
            }}
          />
        </SidebarCard>
      )}

      {post.sourceUrl && (
        <SidebarCard header="Source">
          <LinksA
            links={[
              {
                title: sourceAttribution?.sourceName || "Original Source",
                url: post.sourceUrl.startsWith("http")
                  ? post.sourceUrl
                  : `https://${post.sourceUrl}`,
              },
            ]}
            header=""
          />
        </SidebarCard>
      )}

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
      {banner}
      <NewspaperFrame>
        {headerContent}
        <ArticlePage main={mainContent} sidebar={sidebarContent} />
      </NewspaperFrame>
    </>
  );
}
