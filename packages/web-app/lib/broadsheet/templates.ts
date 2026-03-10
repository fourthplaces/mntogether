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
 * feature-reversed shares the same components as feature (the CSS handles the flip).
 */
const registry: Record<string, Record<string, PostComponent>> = {
  feature: {
    story: FeatureStory,
    notice: FeatureNotice,
    event: FeatureEvent,
    spotlight: FeatureSpotlight,
  },
  'feature-reversed': {
    story: FeatureStory,
    notice: FeatureNotice,
    event: FeatureEvent,
    spotlight: FeatureSpotlight,
  },
  gazette: {
    story: GazetteStory,
    notice: GazetteNotice,
    event: GazetteEvent,
    exchange: GazetteExchange,
    reference: GazetteReference,
    spotlight: GazetteSpotlight,
  },
  ledger: {
    story: LedgerStory,
    notice: LedgerNotice,
    event: LedgerEvent,
    exchange: LedgerExchange,
    reference: LedgerReference,
    spotlight: LedgerSpotlight,
  },
  bulletin: {
    story: BulletinStory,
    notice: BulletinNotice,
    event: BulletinEvent,
    exchange: BulletinExchange,
    reference: BulletinReference,
    spotlight: BulletinSpotlight,
  },
  ticker: {
    story: TickerStory,
    notice: TickerNotice,
    event: TickerEvent,
    exchange: TickerExchange,
  },
  digest: {
    story: DigestStory,
    notice: DigestNotice,
    exchange: DigestExchange,
    spotlight: DigestSpotlight,
  },
  // Specialty templates — each maps to a single post type
  'alert-notice': {
    notice: AlertNotice,
  },
  'pinboard-exchange': {
    exchange: PinboardExchange,
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
    exchange: GenerousExchange,
  },
  'whisper-notice': {
    notice: WhisperNotice,
  },
  'spotlight-local': {
    spotlight: BroadsheetSpotlight,
  },
  'ticker-update': {
    notice: BroadsheetTickerNotice,
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
