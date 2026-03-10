import type { Post } from './types';

/** source.name + source.attribution + location + timestamp joined by middle dots */
export function getMetaLine(post: Post): string {
  const parts: string[] = [];
  if (post.source?.name) parts.push(post.source.name);
  if (post.source?.attribution) parts.push(post.source.attribution);
  if (post.location?.address) parts.push(post.location.address);
  if (post.meta?.timestamp) parts.push(post.meta.timestamp);
  return parts.join(' \u00b7 ');
}

/** source.name + source.attribution + location (no timestamp) */
export function getSourceLine(post: Post): string {
  const parts: string[] = [];
  if (post.source?.name) parts.push(post.source.name);
  if (post.source?.attribution) parts.push(post.source.attribution);
  if (post.location?.address) parts.push(post.location.address);
  return parts.join(' \u00b7 ');
}

/** Derive display label from tagLabel override, tags, or type */
export function getTagLabel(post: Post): string {
  if (post.tagLabel) return post.tagLabel;
  const labels: Record<string, string> = {
    urgent: 'Urgent',
    need: 'Volunteers Needed',
    aid: 'Available',
    action: 'Action',
    person: 'Community Voice',
    business: 'Support Local',
  };
  for (const t of post.tags || []) {
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
  return typeLabels[post.type] || '';
}

/** contact -> formatted HTML string (with bold labels) */
export function getContactDisplay(post: Post): string {
  if (post.contactDisplay) return post.contactDisplay;
  if (!post.contact) return '';
  if (post.contact.phone) return `<strong>Call:</strong> ${post.contact.phone}`;
  if (post.contact.email) return `<strong>Email:</strong> ${post.contact.email}`;
  return post.contact.website || '';
}

/** location + website -> details string */
export function getDetailsLine(post: Post): string {
  const parts: string[] = [];
  if (post.location?.address) parts.push(post.location.address);
  if (post.contact?.website) parts.push(post.contact.website);
  return parts.join(' \u00b7 ');
}
