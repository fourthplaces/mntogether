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
}

// --- Posts ---

export interface PostResult {
  id: string;
  title: string;
  description: string;
  description_markdown: string | null;
  tldr: string | null;
  status: string;
  post_type: string | null;
  category: string | null;
  urgency: string | null;
  location: string | null;
  source_url: string | null;
  website_id: string | null;
  submission_type: string | null;
  created_at: string;
  updated_at: string;
  tags?: TagResult[];
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
  crawl_status: string | null;
}

export interface WebsiteList {
  websites: WebsiteResult[];
  total_count: number;
  has_next_page: boolean;
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
  crawl_status: string | null;
  crawl_attempt_count: number | null;
  max_crawl_retries: number | null;
  last_crawl_started_at: string | null;
  last_crawl_completed_at: string | null;
  pages_crawled_count: number | null;
  max_pages_per_crawl: number | null;
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

export interface DiscoverySourceResult {
  id: string;
  query_id: string;
  domain: string;
  url: string;
  relevance_score: number | null;
  filter_result: string;
  filter_reason: string | null;
  discovered_at: string;
}

// --- Members ---

export interface MemberResult {
  id: string;
  phone_number: string;
  display_name: string | null;
  is_admin: boolean;
  created_at: string;
}

// --- Discovery ---

export interface DiscoveryQuery {
  id: string;
  query_text: string;
  category: string | null;
  is_active: boolean;
  created_at: string;
}

export interface DiscoveryFilterRule {
  id: string;
  query_id: string | null;
  rule_text: string;
  sort_order: number;
  is_active: boolean;
}

export interface DiscoveryRun {
  id: string;
  queries_executed: number;
  total_results: number;
  websites_created: number;
  websites_filtered: number;
  started_at: string;
  completed_at: string | null;
  trigger_type: string;
}

export interface DiscoveryRunResult {
  id: string;
  run_id: string;
  query_id: string;
  domain: string;
  url: string;
  title: string | null;
  snippet: string | null;
  relevance_score: number | null;
  filter_result: string;
  filter_reason: string | null;
  website_id: string | null;
  discovered_at: string;
}

export interface DiscoverySearchResult {
  queries_run: number;
  total_results: number;
  websites_created: number;
  websites_filtered: number;
  run_id: string;
}

// --- Sync ---

export interface SyncBatch {
  id: string;
  resource_type: string;
  source_id: string | null;
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

export interface ExtractionResult {
  job_id: string;
  status: string;
  message: string | null;
}

// --- Resource Link Submission ---

export interface SubmitResourceLinkResult {
  job_id: string;
  status: string;
  message: string | null;
}

// --- Search / Organizations ---

export interface OrganizationResult {
  id: string;
  name: string;
  description: string | null;
  summary: string | null;
  website: string | null;
  phone: string | null;
  primary_address: string | null;
}

export interface OrganizationMatch {
  organization: OrganizationResult;
  similarity_score: number;
}

// --- Post types for public display ---

export type PostType = "service" | "opportunity" | "business" | "professional";
export type Urgency = "urgent" | "high" | "medium" | "low";
export type CapacityStatus = "accepting" | "paused" | "at_capacity";

// --- Public chat message (compatible with ChatPanel) ---

export interface PublicChatMessage {
  id: string;
  chatroom_id: string;
  sender_type: string;
  content: string;
  created_at: string;
}
