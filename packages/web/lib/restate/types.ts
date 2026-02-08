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
}

export interface TagKindResult {
  id: string;
  slug: string;
  display_name: string;
  description: string | null;
  allowed_resource_types: string[];
  tag_count: number;
}

export interface TagKindListResult {
  kinds: TagKindResult[];
}

export interface TagListResult {
  tags: TagResult[];
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

// --- Agents ---

export interface AgentResponse {
  id: string;
  display_name: string;
  role: string;
  status: string;
  created_at: string;
}

export interface AgentListResponse {
  agents: AgentResponse[];
}

export interface AssistantConfigResponse {
  preamble: string;
  config_name: string;
}

export interface CuratorConfigResponse {
  purpose: string;
  audience_roles: string[];
  schedule_discover: string | null;
  schedule_monitor: string | null;
}

export interface SearchQueryResponse {
  id: string;
  query_text: string;
  is_active: boolean;
  sort_order: number;
}

export interface SearchQueryListResponse {
  queries: SearchQueryResponse[];
}

export interface FilterRuleResponse {
  id: string;
  rule_text: string;
  is_active: boolean;
  sort_order: number;
}

export interface FilterRuleListResponse {
  rules: FilterRuleResponse[];
}

export interface AgentTagKindResponse {
  id: string;
  slug: string;
  display_name: string;
}

export interface TagKindListResponse {
  tag_kinds: AgentTagKindResponse[];
}

export interface AgentWebsiteResponse {
  website_id: string;
  domain: string | null;
  discovered_at: string;
}

export interface RunStatResponse {
  stat_key: string;
  stat_value: number;
}

export interface AgentRunResponse {
  id: string;
  step: string;
  trigger_type: string;
  status: string;
  started_at: string;
  completed_at: string | null;
  stats: RunStatResponse[];
}

export interface AgentRunListResponse {
  runs: AgentRunResponse[];
}

export interface AgentDetailResponse {
  agent: AgentResponse;
  assistant_config: AssistantConfigResponse | null;
  curator_config: CuratorConfigResponse | null;
  search_queries: SearchQueryResponse[];
  filter_rules: FilterRuleResponse[];
  required_tag_kinds: AgentTagKindResponse[];
  websites: AgentWebsiteResponse[];
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
}

export interface PublicPostResult {
  id: string;
  title: string;
  tldr: string | null;
  description: string;
  location: string | null;
  source_url: string | null;
  post_type: string;
  category: string;
  created_at: string;
  tags: PublicTagResult[];
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
}

export interface PublicFiltersResult {
  categories: FilterOption[];
  post_types: PostTypeOption[];
}

// --- Public chat message (compatible with ChatPanel) ---

export interface PublicChatMessage {
  id: string;
  chatroom_id: string;
  sender_type: string;
  content: string;
  created_at: string;
}
