/**
 * Transform layer: converts GraphQL BroadsheetPost data into the broadsheet
 * Post interface that broadsheet components consume.
 *
 * This bridges the CMS data model (flat fields, separate contacts/tags arrays)
 * with the broadsheet rendering model (nested objects, computed hint fields).
 */

import type {
  Post,
  PostType,
  PostWeight,
  PostContact,
  PostLocation,
  PostSource,
  PostMeta,
  PostLink,
  PostMedia,
  PostPerson,
  PostDatetime,
  PostItem,
  PostStatus,
} from './types';
import type { BroadsheetPost, BroadsheetContact } from '@/gql/graphql';
import { computeRenderHints } from './render-hints';
import { formatPostDate } from './dates';
import { stripCitations } from './citations';

/**
 * Shape of the per-template config needed by preparePost / enforceBodyLimits.
 * Map is fetched from GraphQL (PostTemplateConfigsQuery) and threaded through
 * BroadsheetRenderer → SlotRenderer → preparePost.
 */
export type PostTemplateConfigMap = Record<string, {
  bodyTarget: number;
  bodyMax: number;
}>;

/**
 * Fallback config used if the API hasn't loaded yet OR the template is
 * unknown. Intentionally conservative — enough to render something
 * without crashing; real limits come from the DB via GraphQL.
 */
const FALLBACK_CONFIG = { bodyTarget: 160, bodyMax: 250 };

/**
 * Build a slug-keyed config map from the PostTemplateConfigsQuery result.
 * Returns undefined if the query hasn't resolved yet (preparePost falls
 * back to FALLBACK_CONFIG per lookup).
 */
export function buildTemplateConfigMap(
  templates: ReadonlyArray<{ slug: string; bodyTarget: number; bodyMax: number }> | undefined | null,
): PostTemplateConfigMap | undefined {
  if (!templates || templates.length === 0) return undefined;
  const map: PostTemplateConfigMap = {};
  for (const t of templates) {
    map[t.slug] = { bodyTarget: t.bodyTarget, bodyMax: t.bodyMax };
  }
  return map;
}

/**
 * Convert a GraphQL BroadsheetPost + its assigned post template
 * into the broadsheet Post interface for component rendering.
 *
 * `templateConfigs` is the result of PostTemplateConfigsQuery, passed in
 * by the page (BroadsheetRenderer threads it through). When missing or
 * the template slug isn't found, a sensible fallback is used.
 */
export function preparePost(
  gqlPost: BroadsheetPost,
  postTemplate: string,
  isAnchor?: boolean,
  templateConfigs?: PostTemplateConfigMap,
): Post {
  const config = templateConfigs?.[postTemplate] ?? FALLBACK_CONFIG;
  const isFeature = postTemplate === 'feature' || postTemplate === 'feature-reversed';

  // Tags: extract tag values as string[] for the broadsheet type system
  const tagValues = gqlPost.tags.map((t) => t.value);
  // Check urgent notes OR urgency field → add 'urgent' tag
  if (
    (gqlPost.urgentNotes.length > 0 || gqlPost.urgency === 'urgent') &&
    !tagValues.includes('urgent')
  ) {
    tagValues.push('urgent');
  }

  // Contacts: flatten into PostContact shape
  const contact = buildContact(gqlPost.contacts);

  // Body: use weight-specific text from Root Signal if available, else fall back to description.
  // Anchor posts in stacked layouts need more body text to fill their wider column,
  // so we bump them up one tier (medium → heavy).
  const rawBodyWithCitations = (isAnchor
    ? (gqlPost.bodyHeavy ?? selectWeightBody(gqlPost, postTemplate))
    : selectWeightBody(gqlPost, postTemplate)
  ) ?? gqlPost.bodyRaw;
  // Broadsheet tiles render short snippets — [signal:UUID] superscripts
  // are noise at that scale. Detail pages render citations inline via
  // CitationMarkdown; here we strip them so card copy stays clean.
  const rawBody = rawBodyWithCitations ? stripCitations(rawBodyWithCitations) : rawBodyWithCitations;
  // Anchors use bodyHeavy, so enforce against feature-level limits (not the template's own)
  const { html: bodyHtml, compact } = enforceBodyLimits(
    rawBody,
    isAnchor ? 'feature' : postTemplate,
    templateConfigs,
  );
  const paragraphs = splitParagraphs(bodyHtml);

  // Compute clamp based on template body target (chars → approximate line count)
  const clamp = computeClamp(postTemplate);

  // Derive tag label from tags
  const tagLabel = deriveTagLabel(tagValues, gqlPost.postType as PostType);

  // Field groups: prefer real data from field group tables, fall back to flat fields
  const media: PostMedia | undefined = gqlPost.media?.length
    ? { image: gqlPost.media[0].imageUrl ?? undefined, caption: gqlPost.media[0].caption ?? undefined, credit: gqlPost.media[0].credit ?? undefined }
    : undefined;

  const person: PostPerson | undefined = gqlPost.person
    ? { name: gqlPost.person.name ?? undefined, role: gqlPost.person.role ?? undefined, bio: gqlPost.person.bio ?? undefined, photo: gqlPost.person.photoUrl ?? undefined, quote: gqlPost.person.quote ?? undefined }
    : undefined;

  const source: PostSource | undefined = gqlPost.sourceAttribution
    ? { name: gqlPost.sourceAttribution.sourceName ?? undefined, attribution: gqlPost.sourceAttribution.attribution ?? undefined }
    : gqlPost.organizationName
      ? { name: gqlPost.organizationName }
      : undefined;

  const meta: PostMeta | undefined = gqlPost.meta
    ? { kicker: gqlPost.meta.kicker ?? undefined, byline: gqlPost.meta.byline ?? undefined, timestamp: gqlPost.meta.timestamp ?? undefined, updated: gqlPost.meta.updated ?? undefined, deck: gqlPost.meta.deck ?? undefined }
    : buildMeta(gqlPost);

  const link: PostLink | undefined = gqlPost.link
    ? { label: gqlPost.link.label ?? undefined, url: gqlPost.link.url ?? undefined, deadline: gqlPost.link.deadline ?? undefined }
    : gqlPost.sourceUrl
      ? { url: gqlPost.sourceUrl }
      : undefined;

  const items: PostItem[] | undefined = gqlPost.items?.length
    ? gqlPost.items.map((i) => ({ name: i.name, detail: i.detail ?? '' }))
    : undefined;

  const datetime: PostDatetime | undefined = gqlPost.datetime
    ? { start: gqlPost.datetime.start ?? undefined, end: gqlPost.datetime.end ?? undefined, cost: gqlPost.datetime.cost ?? undefined, recurring: gqlPost.datetime.recurring ?? undefined }
    : undefined;

  const postStatus: PostStatus | undefined = gqlPost.postStatus
    ? { state: gqlPost.postStatus.state ?? undefined, verified: gqlPost.postStatus.verified ?? undefined }
    : undefined;

  const schedule = gqlPost.schedule?.length
    ? { entries: gqlPost.schedule.map((e) => ({ day: e.day, opens: e.opens, closes: e.closes })) }
    : undefined;

  // Build the broadsheet Post
  const post: Post = {
    id: gqlPost.id,
    type: gqlPost.postType as PostType,
    tags: tagValues,
    weight: gqlPost.weight as PostWeight,
    priority: 0, // Not exposed in public API — default to 0
    // Modifiers — overlays on top of type-driven visual variants
    urgent: ((gqlPost as any).isUrgent ?? gqlPost.urgency === 'urgent') || undefined,
    // Pencil marks are hidden on dark backgrounds (urgent, action CTA) per style guide.
    // Clear at the data layer so components don't need to check individually.
    pencilMark: ((gqlPost as any).isUrgent || gqlPost.urgency === 'urgent')
      ? undefined
      : ((gqlPost as any).pencilMark ?? undefined),

    title: gqlPost.title,
    body: bodyHtml,

    // Field groups
    media,
    contact: contact || undefined,
    location: gqlPost.location
      ? { address: gqlPost.location } as PostLocation
      : undefined,
    person,
    source,
    meta,
    link,
    items,
    datetime,
    status: postStatus,
    schedule,

    // Renderer hints
    paragraphs: isFeature ? paragraphs : undefined,
    cols: isFeature && isAnchor && paragraphs.length >= 4 ? 2 : undefined,
    dropCap: isFeature,
    // Features use <MRichBody> (which ignores clamp) so undefined is fine.
    // Everyone else — including anchor cells — gets the template's configured
    // clamp value. The old logic set clamp=0 for anchors on the theory that
    // wider anchor columns "don't need clamping", but 0 isn't a valid CSS
    // `.clamp-N` class, so anchor bodies rendered un-clamped and overflowed
    // their cells (visible on alert-urgent, gaz-story, gaz-request).
    clamp: isFeature ? undefined : clamp,
    tagLabel,
    // readMore is the external source URL (e.g. the original newspaper
    // article). Internal navigation to the post detail page is handled
    // by the title link, not readMore — those are semantically distinct
    // destinations.
    readMore: gqlPost.sourceUrl || undefined,
    compact,
    deck: meta?.deck,

    // Feature-level image/caption/credit shorthand (backward compat)
    image: media?.image,
    caption: media?.caption,
    credit: media?.credit,
  };

  // Compute render hints from field group data and merge onto post
  const hints = computeRenderHints(post);
  if (hints.month !== undefined) post.month = hints.month;
  if (hints.day !== undefined) post.day = hints.day;
  if (hints.dow !== undefined) post.dow = hints.dow;
  if (hints.when !== undefined) post.when = hints.when;
  if (hints.circleLabel !== undefined) post.circleLabel = hints.circleLabel;
  if (hints.count !== undefined) post.count = hints.count;
  if (hints.tagline !== undefined) post.tagline = hints.tagline;
  if (hints.pullQuote !== undefined) post.pullQuote = hints.pullQuote;
  if (hints.date !== undefined) post.date = hints.date;

  return post;
}

// =============================================================================
// Helpers
// =============================================================================

function buildContact(
  contacts: readonly BroadsheetContact[]
): PostContact | null {
  if (!contacts.length) return null;

  const result: PostContact = {};
  for (const c of contacts) {
    switch (c.contactType) {
      case 'phone':
        result.phone = result.phone ?? c.contactValue;
        break;
      case 'email':
        result.email = result.email ?? c.contactValue;
        break;
      case 'website':
      case 'booking_url':
        result.website = result.website ?? c.contactValue;
        break;
    }
  }

  return result.phone || result.email || result.website ? result : null;
}

function buildMeta(gqlPost: BroadsheetPost): PostMeta | undefined {
  const parts: Partial<PostMeta> = {};

  if (gqlPost.publishedAt) {
    // Store the raw ISO string — render-hints will format it via formatPostDate()
    parts.timestamp = gqlPost.publishedAt;
  }

  return Object.keys(parts).length ? (parts as PostMeta) : undefined;
}

function splitParagraphs(html: string): string[] {
  // If the body contains <p> tags, split on them
  if (html.includes('<p>') || html.includes('<p ')) {
    return html
      .split(/<\/p>\s*<p[^>]*>|<\/p>|<p[^>]*>/)
      .map((s) => s.trim())
      .filter(Boolean);
  }
  // Otherwise split on double newlines
  return html
    .split(/\n\n+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

function computeClamp(postTemplate: string): number | undefined {
  // Map templates to approximate CSS clamp line counts
  switch (postTemplate) {
    case 'ticker':
    case 'ticker-update':
      return 2;
    case 'bulletin':
    case 'whisper-notice':
    case 'quick-ref':
      return 3;
    case 'digest':
    case 'ledger':
      return 4;
    case 'gazette':
    case 'alert-notice':
    case 'pinboard-exchange':
    case 'card-event':
    case 'generous-exchange':
    case 'directory-ref':
    case 'spotlight-local':
      return 6;
    default:
      return undefined; // features don't clamp
  }
}

function deriveTagLabel(tags: string[], postType: PostType): string {
  const labels: Record<string, string> = {
    urgent: 'Urgent',
    need: 'Volunteers Needed',
    aid: 'Available',
    action: 'Action',
    person: 'Community Voice',
    business: 'Support Local',
  };

  for (const t of tags) {
    if (labels[t]) return labels[t];
  }

  // 9-type system (post-migration 216)
  const typeLabels: Record<string, string> = {
    story: 'Story',
    update: 'Update',
    action: 'Action',
    event: 'Event',
    need: 'Needed',
    aid: 'Offer',
    person: 'Community Voice',
    business: 'Support Local',
    reference: 'Reference',
  };

  return typeLabels[postType] || '';
}

// Template weight tier mapping — which weight tier does each template belong to?
const TEMPLATE_WEIGHT_TIER: Record<string, 'heavy' | 'medium' | 'light'> = {
  feature: 'heavy',
  'feature-reversed': 'heavy',
  gazette: 'medium',
  bulletin: 'medium',
  ledger: 'light',
  ticker: 'light',
  digest: 'light',
  'alert-notice': 'medium',
  'pinboard-exchange': 'medium',
  'card-event': 'medium',
  'quick-ref': 'light',
  'directory-ref': 'medium',
  'generous-exchange': 'medium',
  'whisper-notice': 'light',
  'spotlight-local': 'medium',
  'ticker-update': 'light',
};

/**
 * Select the weight-appropriate body text from Root Signal data.
 * Returns null if no weight-specific body exists for this template's tier.
 */
/**
 * Enforce body text min/max per template config.
 * Truncates at word boundary if over bodyMax; flags compact if under bodyMin.
 */
function enforceBodyLimits(
  body: string,
  postTemplate: string,
  templateConfigs?: PostTemplateConfigMap,
): { html: string; compact?: boolean } {
  const config = templateConfigs?.[postTemplate] ?? FALLBACK_CONFIG;
  if (config.bodyMax === 0) return { html: body };

  // Strip HTML tags for character counting
  const plain = body.replace(/<[^>]*>/g, '');
  const len = plain.length;

  // Truncate if over max
  let html = body;
  if (len > config.bodyMax) {
    // Find last space at or before bodyMax in plaintext
    let cutoff = config.bodyMax;
    const spaceIdx = plain.lastIndexOf(' ', cutoff);
    if (spaceIdx > cutoff * 0.7) cutoff = spaceIdx;
    // Map plaintext position back to HTML — simple approach: strip tags, truncate, done
    // Since body is typically plain text or simple HTML, truncate the plain version
    html = plain.slice(0, cutoff).trimEnd() + '\u2026';
  }

  // Flag compact if under minimum (60% of target)
  const bodyMin = Math.floor(config.bodyTarget * 0.6);
  const compact = len < bodyMin ? true : undefined;

  return { html, compact };
}

function selectWeightBody(
  gqlPost: BroadsheetPost,
  postTemplate: string
): string | null {
  const tier = TEMPLATE_WEIGHT_TIER[postTemplate] ?? 'medium';

  switch (tier) {
    case 'heavy':
      return gqlPost.bodyHeavy ?? null;
    case 'medium':
      return gqlPost.bodyMedium ?? null;
    case 'light':
      return gqlPost.bodyLight ?? null;
    default:
      return null;
  }
}
