// TypeScript types matching Rust Restate response structs
// All fields are snake_case to match Rust serde defaults.

// --- Auth ---

export interface OtpSent {
  phone_number: string;
  success: boolean;
}

export interface OtpVerified {
  member_id: string;
  phone_number: string;
  is_admin: boolean;
  token: string;
}

// --- Tags ---

export interface TagResult {
  id: string;
  kind: string;
  value: string;
  display_name: string | null;
  color: string | null;
  description: string | null;
  emoji: string | null;
}

export interface TagKindResult {
  id: string;
  slug: string;
  display_name: string;
  description: string | null;
  allowed_resource_types: string[];
  required: boolean;
  is_public: boolean;
  tag_count: number;
}

export interface TagKindListResult {
  kinds: TagKindResult[];
}

export interface TagListResult {
  tags: TagResult[];
}

// --- Posts ---

export interface SubmittedByInfo {
  submitter_type: string;
  agent_id: string | null;
  agent_name: string | null;
}

export interface PostScheduleResult {
  id: string;
  day_of_week: number | null;
  opens_at: string | null;
  closes_at: string | null;
  timezone: string;
  notes: string | null;
  rrule: string | null;
  dtstart: string | null;
  dtend: string | null;
  is_all_day: boolean;
  duration_minutes: number | null;
}

export interface PostContactResult {
  id: string;
  contact_type: string;
  contact_value: string;
  contact_label: string | null;
}

export interface PostResult {
  id: string;
  title: string;
  description: string;
  description_markdown: string | null;
  summary: string | null;
  status: string;
  post_type: string | null;
  category: string | null;
  urgency: string | null;
  location: string | null;
  source_url: string | null;
  submission_type: string | null;
  created_at: string;
  updated_at: string;
  published_at?: string | null;
  tags?: TagResult[];
  submitted_by?: SubmittedByInfo;
  schedules?: PostScheduleResult[];
  contacts?: PostContactResult[];
  organization_id?: string;
  organization_name?: string;
  has_urgent_notes?: boolean;
}

export interface PostList {
  posts: PostResult[];
  total_count: number;
  has_next_page: boolean;
  has_previous_page: boolean;
}

export interface PostStats {
  total: number;
  services: number;
  opportunities: number;
  businesses: number;
  user_submitted: number;
  scraped: number;
}

export interface SourcePage {
  url: string;
  title: string | null;
  fetched_at: string;
  content: string;
}

export interface PostDetail extends PostResult {
  source_pages: SourcePage[];
}

// --- Websites ---

export interface WebsiteResult {
  id: string;
  domain: string;
  status: string;
  active: boolean;
  created_at: string | null;
  crawl_count: number | null;
  post_count: number | null;
  last_crawled_at: string | null;
  organization_id: string | null;
}

export interface WebsiteList {
  websites: WebsiteResult[];
  total_count: number;
  has_next_page: boolean;
}

export interface SubmitWebsiteRequest {
  url: string;
}

export interface SnapshotResult {
  url: string;
  site_url: string;
  title: string | null;
  content: string;
  fetched_at: string;
  listings_count: number;
}

export interface WebsiteDetail {
  id: string;
  domain: string;
  status: string;
  submitted_by: string | null;
  submitter_type: string;
  last_scraped_at: string | null;
  snapshots_count: number;
  listings_count: number;
  created_at: string;
  snapshots: SnapshotResult[];
  listings: PostResult[];
}

export interface AssessmentResult {
  id: string;
  website_id: string;
  assessment_markdown: string;
  confidence_score: number | null;
}

export interface OptionalAssessmentResult {
  assessment: AssessmentResult | null;
}

// --- Members ---

export interface MemberResult {
  id: string;
  phone_number: string;
  display_name: string | null;
  is_admin: boolean;
  created_at: string;
}

// --- Search Queries ---

export interface SearchQueryResult {
  id: string;
  query_text: string;
  is_active: boolean;
  sort_order: number;
}

export interface SearchQueryListResult {
  queries: SearchQueryResult[];
}

// --- Sync ---

export interface SyncBatch {
  id: string;
  resource_type: string;
  source_id: string | null;
  source_name: string | null;
  status: string;
  summary: string | null;
  proposal_count: number;
  approved_count: number;
  rejected_count: number;
  created_at: string;
  reviewed_at: string | null;
}

export interface SyncProposal {
  id: string;
  batch_id: string;
  operation: string;
  status: string;
  entity_type: string;
  draft_entity_id: string | null;
  target_entity_id: string | null;
  reason: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  created_at: string;
  draft_title: string | null;
  target_title: string | null;
  merge_source_ids: string[];
  merge_source_titles: string[];
}

export interface EntityProposal {
  id: string;
  batch_id: string;
  operation: string;
  status: string;
  entity_type: string;
  draft_entity_id: string | null;
  target_entity_id: string | null;
  reason: string | null;
  created_at: string;
}

export interface EntityProposalListResult {
  proposals: EntityProposal[];
}

// --- Providers ---

export interface ProviderResult {
  id: string;
  name: string;
  domain: string | null;
  status: string;
  created_at: string;
}

// --- Chatrooms ---

export interface ChatMessage {
  id: string;
  chatroom_id: string;
  sender_type: string;
  content: string;
  created_at: string;
}

export interface ChatroomResult {
  id: string;
  title: string | null;
  created_at: string;
  message_count: number;
}

// --- Extraction ---

export interface ExtractionPageResult {
  url: string;
  content: string | null;
}

export interface ExtractionPageListResult {
  pages: ExtractionPageResult[];
}

export interface ExtractionPageCount {
  count: number;
}

// --- Resource Link Submission ---

export interface SubmitResourceLinkResult {
  job_id: string;
  status: string;
  message: string | null;
}

// --- Organizations ---

export interface OrganizationResult {
  id: string;
  name: string;
  description: string | null;
  status: string;
  website_count: number;
  social_profile_count: number;
  created_at: string;
  updated_at: string;
}

export interface OrganizationListResult {
  organizations: OrganizationResult[];
}

export interface OrganizationDetailResult {
  id: string;
  name: string;
  description: string | null;
  posts: PublicPostResult[];
}

// --- Notes ---

export interface LinkedPostResult {
  id: string;
  title: string;
}

export interface NoteResult {
  id: string;
  content: string;
  severity: string;
  source_url: string | null;
  source_id: string | null;
  source_type: string | null;
  is_public: boolean;
  created_by: string;
  expired_at: string | null;
  created_at: string;
  updated_at: string;
  linked_posts?: LinkedPostResult[];
}

export interface NoteListResult {
  notes: NoteResult[];
}

// --- Sources (unified) ---

export interface SourceResult {
  id: string;
  source_type: string;
  identifier: string;
  url: string | null;
  status: string;
  active: boolean;
  organization_id: string | null;
  organization_name: string | null;
  scrape_frequency_hours: number;
  last_scraped_at: string | null;
  post_count: number | null;
  created_at: string;
  updated_at: string;
}

export interface SourceListResult {
  sources: SourceResult[];
  total_count: number;
  has_next_page: boolean;
  has_previous_page: boolean;
}

export interface SourceObjectResult {
  id: string;
  source_type: string;
  identifier: string;
  url: string | null;
  status: string;
  active: boolean;
  created_at: string | null;
  last_scraped_at: string | null;
  organization_id: string | null;
}

// --- Social Profiles ---

export interface SocialProfileResult {
  id: string;
  organization_id: string;
  platform: string;
  handle: string;
  url: string | null;
  scrape_frequency_hours: number;
  last_scraped_at: string | null;
  active: boolean;
  created_at: string;
}

export interface SocialProfileListResult {
  profiles: SocialProfileResult[];
}

// --- Post types for public display ---

export type PostType = "seeking" | "offering" | "announcement" | "service" | "opportunity" | "business" | "professional";
export type Urgency = "urgent" | "high" | "medium" | "low";
export type CapacityStatus = "accepting" | "paused" | "at_capacity";

// --- Jobs ---

export interface JobResult {
  id: string;
  workflow_name: string;
  workflow_key: string;
  status: string;
  progress: string | null;
  created_at: string | null;
  modified_at: string | null;
  completed_at: string | null;
  completion_result: string | null;
  website_domain: string | null;
  website_id: string | null;
}

export interface JobListResult {
  jobs: JobResult[];
}

// --- Public home page ---

export interface PublicListRequest {
  post_type?: string;
  category?: string;
  limit?: number;
  offset?: number;
}

export interface PublicTagResult {
  kind: string;
  value: string;
  display_name: string | null;
  color: string | null;
}

export interface PublicPostResult {
  id: string;
  title: string;
  summary: string | null;
  description: string;
  location: string | null;
  source_url: string | null;
  post_type: string;
  category: string;
  created_at: string;
  published_at?: string | null;
  tags: PublicTagResult[];
  has_urgent_notes?: boolean;
}

export interface PublicListResult {
  posts: PublicPostResult[];
  total_count: number;
}

export interface FilterOption {
  value: string;
  display_name: string;
  count: number;
}

export interface PostTypeOption {
  value: string;
  display_name: string;
  description: string | null;
  color: string | null;
  emoji: string | null;
}

export interface PublicFiltersResult {
  categories: FilterOption[];
  post_types: PostTypeOption[];
}

// --- Comments ---

export interface CommentMessage {
  id: string;
  container_id: string;
  role: string;
  content: string;
  parent_message_id: string | null;
  created_at: string;
}

export interface CommentListResult {
  messages: CommentMessage[];
}

// --- Public chat message (compatible with ChatPanel) ---

export interface PublicChatMessage {
  id: string;
  chatroom_id: string;
  sender_type: string;
  content: string;
  created_at: string;
}
