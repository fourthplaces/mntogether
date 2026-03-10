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
} from './types';
import type { BroadsheetPost, BroadsheetContact } from '@/gql/graphql';

// Post template configs from the CMS — mirrors post_template_configs seed data
const TEMPLATE_CONFIGS: Record<string, { bodyTarget: number; bodyMax: number }> = {
  feature:          { bodyTarget: 400, bodyMax: 600 },
  'feature-reversed': { bodyTarget: 350, bodyMax: 500 },
  gazette:          { bodyTarget: 160, bodyMax: 250 },
  ledger:           { bodyTarget: 120, bodyMax: 180 },
  bulletin:         { bodyTarget: 80, bodyMax: 120 },
  ticker:           { bodyTarget: 60, bodyMax: 80 },
  digest:           { bodyTarget: 100, bodyMax: 150 },
};

/**
 * Convert a GraphQL BroadsheetPost + its assigned post template
 * into the broadsheet Post interface for component rendering.
 */
export function preparePost(
  gqlPost: BroadsheetPost,
  postTemplate: string
): Post {
  const config = TEMPLATE_CONFIGS[postTemplate] ?? TEMPLATE_CONFIGS.gazette;
  const isFeature = postTemplate === 'feature' || postTemplate === 'feature-reversed';

  // Tags: extract tag values as string[] for the broadsheet type system
  const tagValues = gqlPost.tags.map((t) => t.value);
  // Also check urgent notes → add 'urgent' tag
  if (gqlPost.urgentNotes.length > 0 && !tagValues.includes('urgent')) {
    tagValues.push('urgent');
  }

  // Contacts: flatten into PostContact shape
  const contact = buildContact(gqlPost.contacts);

  // Body: truncate description to bodyMax, split into paragraphs for features
  const bodyHtml = gqlPost.description;
  const paragraphs = splitParagraphs(bodyHtml);

  // Compute clamp based on template body target (chars → approximate line count)
  const clamp = computeClamp(postTemplate);

  // Derive tag label from tags
  const tagLabel = deriveTagLabel(tagValues, gqlPost.postType as PostType);

  // Build the broadsheet Post
  const post: Post = {
    id: gqlPost.id,
    type: gqlPost.postType as PostType,
    tags: tagValues,
    weight: gqlPost.weight as PostWeight,
    priority: 0, // Not exposed in public API — default to 0

    title: gqlPost.title,
    body: bodyHtml,

    // Nested objects
    contact: contact || undefined,
    location: gqlPost.location
      ? { address: gqlPost.location } as PostLocation
      : undefined,
    source: gqlPost.organizationName
      ? { name: gqlPost.organizationName } as PostSource
      : undefined,
    meta: buildMeta(gqlPost),
    link: gqlPost.sourceUrl
      ? { url: gqlPost.sourceUrl } as PostLink
      : undefined,

    // Renderer hints
    paragraphs: isFeature ? paragraphs : undefined,
    cols: isFeature && paragraphs.length >= 4 ? 2 : undefined,
    dropCap: isFeature,
    clamp: isFeature ? undefined : clamp,
    tagLabel,
    readMore: gqlPost.sourceUrl || undefined,
  };

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
    const d = new Date(gqlPost.publishedAt);
    if (!isNaN(d.getTime())) {
      parts.timestamp = d.toLocaleDateString('en-US', {
        month: 'short',
        day: 'numeric',
        year: 'numeric',
      });
    }
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
      return 2;
    case 'bulletin':
      return 3;
    case 'digest':
      return 4;
    case 'ledger':
      return 4;
    case 'gazette':
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

  const typeLabels: Record<string, string> = {
    story: 'Story',
    notice: 'Update',
    event: 'Event',
    exchange: 'Exchange',
    spotlight: 'Local',
    reference: 'Reference',
  };

  return typeLabels[postType] || '';
}
