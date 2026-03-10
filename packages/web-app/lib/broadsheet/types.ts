export type PostType = 'story' | 'notice' | 'exchange' | 'event' | 'spotlight' | 'reference';
export type PostWeight = 'heavy' | 'medium' | 'light';
export type PostFamily = 'gazette' | 'bulletin' | 'ledger';
export type FeatureVariant = 'story' | 'editorial' | 'hero' | 'photo' | 'notice' | 'event' | 'spotlight';
export type DigestType = 'story' | 'notice' | 'exchange' | 'spotlight';
export type RowVariant = 'lead' | 'lead-stack' | 'pair' | 'pair-stack' | 'trio' | 'trio-mixed' | 'full';
export type CellSpan = 1 | 2 | 3 | 4 | 6;

export interface PostMedia {
  image?: string;
  caption?: string;
  credit?: string;
}

export interface PostContact {
  phone?: string;
  email?: string;
  website?: string;
}

export interface PostLocation {
  address?: string;
  city?: string;
}

export interface PostDatetime {
  start?: string;
  end?: string;
  cost?: string;
  recurring?: boolean;
}

export interface PostPerson {
  name?: string;
  role?: string;
  photo?: string;
  bio?: string;
  quote?: string;
}

export interface PostSource {
  name?: string;
  attribution?: string;
}

export interface PostMeta {
  kicker?: string;
  byline?: string;
  timestamp?: string;
  updated?: string;
  deck?: string;
}

export interface PostLink {
  label?: string;
  url?: string;
  deadline?: string;
}

export interface PostItem {
  name: string;
  detail: string;
}

export interface PostStatus {
  state?: string;
  verified?: string;
}

export interface PostScheduleEntry {
  day: string;
  opens: string;
  closes: string;
}

export interface Post {
  id: string;
  type: PostType;
  tags: string[];
  weight: PostWeight;
  priority: number;
  title: string;
  body: string;
  media?: PostMedia;
  contact?: PostContact;
  location?: PostLocation;
  schedule?: { entries: PostScheduleEntry[] };
  items?: PostItem[];
  status?: PostStatus;
  datetime?: PostDatetime;
  person?: PostPerson;
  source?: PostSource;
  meta?: PostMeta;
  link?: PostLink;

  // Renderer hint fields — computed at render time from spec data
  paragraphs?: string[];
  cols?: number;
  dropCap?: boolean;
  pullQuote?: string;
  deck?: string;
  clamp?: number;
  tagLabel?: string;
  contactDisplay?: string;
  month?: string;
  day?: string;
  dow?: string;
  when?: string;
  circleLabel?: string;
  date?: string;
  count?: string;
  tagline?: string;
  readMore?: string;

  // Feature-specific
  image?: string;
  caption?: string;
  credit?: string;

  // Widget-specific fields used in section breaks
  sub?: string;
}
