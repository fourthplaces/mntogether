/**
 * Template registry: maps (postTemplate, postType) → React component.
 *
 * The CMS assigns each post a "post template" (feature, gazette, ledger, etc.)
 * and posts have a "post type" (story, notice, event, etc.). This registry
 * resolves the correct broadsheet component for each combination.
 *
 * Fallback chain:
 *   1. Exact match: template + type
 *   2. Gazette variant of the type (gazette is the most versatile family)
 *   3. GazetteStory (ultimate default)
 */

import type { ComponentType } from 'react';
import type { Post } from './types';

// Lazy import to allow tree-shaking — only referenced components are bundled
import {
  // Feature family
  FeatureStory,
  FeatureNotice,
  FeatureEvent,
  FeatureSpotlight,
  // Gazette family
  GazetteStory,
  GazetteNotice,
  GazetteEvent,
  GazetteExchange,
  GazetteReference,
  GazetteSpotlight,
  // Ledger family
  LedgerStory,
  LedgerNotice,
  LedgerEvent,
  LedgerExchange,
  LedgerReference,
  LedgerSpotlight,
  // Bulletin family
  BulletinStory,
  BulletinNotice,
  BulletinEvent,
  BulletinExchange,
  BulletinReference,
  BulletinSpotlight,
  // Ticker family
  TickerStory,
  TickerNotice,
  TickerEvent,
  TickerExchange,
  // Digest family
  DigestStory,
  DigestNotice,
  DigestExchange,
  DigestSpotlight,
  // Specialty components
  AlertNotice,
  PinboardExchange,
  CardEvent,
  QuickRef,
  DirectoryRef,
  GenerousExchange,
  WhisperNotice,
  BroadsheetSpotlight,
  BroadsheetTickerNotice,
} from '@/components/broadsheet';

type PostComponent = ComponentType<{ data: Post }>;

/**
 * Registry: postTemplate → postType → Component
 *
 * Uses the 9-type model (story | update | action | event | need | aid | person | business | reference).
 * Components that handle multiple types (e.g. GazetteNotice handles update + action)
 * branch internally on `d.type`.
 *
 * feature-reversed shares the same components as feature (the CSS handles the flip).
 */
const registry: Record<string, Record<string, PostComponent>> = {
  feature: {
    story: FeatureStory,
    update: FeatureNotice,
    action: FeatureNotice,
    event: FeatureEvent,
    person: FeatureSpotlight,
    business: FeatureSpotlight,
  },
  'feature-reversed': {
    story: FeatureStory,
    update: FeatureNotice,
    action: FeatureNotice,
    event: FeatureEvent,
    person: FeatureSpotlight,
    business: FeatureSpotlight,
  },
  gazette: {
    story: GazetteStory,
    update: GazetteNotice,
    action: GazetteNotice,
    event: GazetteEvent,
    need: GazetteExchange,
    aid: GazetteExchange,
    reference: GazetteReference,
    person: GazetteSpotlight,
    business: GazetteSpotlight,
  },
  ledger: {
    story: LedgerStory,
    update: LedgerNotice,
    action: LedgerNotice,
    event: LedgerEvent,
    need: LedgerExchange,
    aid: LedgerExchange,
    reference: LedgerReference,
    person: LedgerSpotlight,
    business: LedgerSpotlight,
  },
  bulletin: {
    story: BulletinStory,
    update: BulletinNotice,
    action: BulletinNotice,
    event: BulletinEvent,
    need: BulletinExchange,
    aid: BulletinExchange,
    reference: BulletinReference,
    person: BulletinSpotlight,
    business: BulletinSpotlight,
  },
  ticker: {
    story: TickerStory,
    update: TickerNotice,
    action: TickerNotice,
    event: TickerEvent,
    need: TickerExchange,
    aid: TickerExchange,
  },
  digest: {
    story: DigestStory,
    update: DigestNotice,
    action: DigestNotice,
    need: DigestExchange,
    aid: DigestExchange,
    person: DigestSpotlight,
    business: DigestSpotlight,
  },
  // Specialty templates — each maps to a few specific post types
  'alert-notice': {
    update: AlertNotice,
    action: AlertNotice,
  },
  'pinboard-exchange': {
    need: PinboardExchange,
    aid: PinboardExchange,
  },
  'card-event': {
    event: CardEvent,
  },
  'quick-ref': {
    reference: QuickRef,
  },
  'directory-ref': {
    reference: DirectoryRef,
  },
  'generous-exchange': {
    need: GenerousExchange,
    aid: GenerousExchange,
  },
  'whisper-notice': {
    update: WhisperNotice,
  },
  'spotlight-local': {
    person: BroadsheetSpotlight,
    business: BroadsheetSpotlight,
  },
  'ticker-update': {
    update: BroadsheetTickerNotice,
  },
};

/**
 * Resolve the correct post component for a given template + type combination.
 *
 * Fallback chain:
 *   1. registry[postTemplate][postType]  — exact match
 *   2. registry.gazette[postType]        — gazette is the universal family
 *   3. GazetteStory                      — ultimate default
 */
export function resolveTemplate(
  postTemplate: string,
  postType: string
): PostComponent {
  // 1. Exact match
  const templateMap = registry[postTemplate];
  if (templateMap?.[postType]) {
    return templateMap[postType];
  }

  // 2. Gazette fallback for this type
  const gazetteFallback = registry.gazette[postType];
  if (gazetteFallback) {
    return gazetteFallback;
  }

  // 3. Ultimate default
  return GazetteStory;
}
