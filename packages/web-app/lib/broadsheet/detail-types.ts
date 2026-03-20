/* ═══════════════════════════════════════════════
   DETAIL PAGE TYPES
   Interfaces for post detail page components.
   ═══════════════════════════════════════════════ */

export interface PhotoData {
  src: string;
  alt?: string;
  caption: string;
  credit: string;
}

export interface AudioData {
  title: string;
  duration: string;
  credit?: string;
  currentTime?: string;
  excerpt?: string;
}

export interface LinkData {
  title: string;
  url: string;
}

export interface AddressData {
  street: string;
  city: string;
  state: string;
  zip: string;
  directionsUrl?: string;
}

export interface PhoneData {
  number: string;
  display?: string;
  label?: string;
}

export interface EmailData {
  address: string;
  label?: string;
}

export interface WebsiteData {
  url: string;
  label?: string;
}

export interface RelatedArticle {
  id: string;
  kicker: string;
  title: string;
  meta?: string;
  color?: string;
}

export interface ResourceItem {
  name: string;
  detail: string;
}

export type TitleSize = 'story' | 'event' | 'reference' | 'notice' | 'spotlight' | 'exchange';
