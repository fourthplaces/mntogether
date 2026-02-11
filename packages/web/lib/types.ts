// TypeScript types for API responses

// ============================================================================
// Common Types
// ============================================================================

export interface PageInfo {
  hasNextPage: boolean;
  hasPreviousPage?: boolean;
  startCursor?: string | null;
  endCursor?: string | null;
}

export interface PaginatedResult<T> {
  nodes: T[];
  pageInfo: PageInfo;
  totalCount: number;
}

export interface Tag {
  id: string;
  kind: string;
  value: string;
  displayName?: string;
}

export interface ContactInfo {
  email?: string;
  phone?: string;
  website?: string;
}

// ============================================================================
// Post Types
// ============================================================================

export type PostType = "service" | "opportunity" | "business" | "professional";
export type PostStatus = "PENDING_APPROVAL" | "ACTIVE" | "REJECTED" | "EXPIRED" | "ARCHIVED";
export type CapacityStatus = "accepting" | "paused" | "at_capacity";
export type SubmissionType = "SCRAPED" | "MANUAL" | "USER_SUBMITTED";
export type Urgency = "urgent" | "high" | "medium" | "low";

export interface Post {
  id: string;
  title: string;
  summary?: string;
  description: string;
  descriptionMarkdown?: string;
  postType?: PostType;
  category?: string;
  capacityStatus?: CapacityStatus;
  urgency?: Urgency;
  status: PostStatus;
  location?: string;
  sourceUrl?: string;
  submissionType?: SubmissionType;
  tags?: Tag[];
  createdAt: string;
  updatedAt?: string;

  // Service-specific fields
  requiresIdentification?: boolean;
  requiresAppointment?: boolean;
  walkInsAccepted?: boolean;
  remoteAvailable?: boolean;
  inPersonAvailable?: boolean;
  homeVisitsAvailable?: boolean;
  wheelchairAccessible?: boolean;
  interpretationAvailable?: boolean;
  freeService?: boolean;
  slidingScaleFees?: boolean;
  acceptsInsurance?: boolean;
  eveningHours?: boolean;
  weekendHours?: boolean;

  // Opportunity-specific fields
  opportunityType?: string;
  timeCommitment?: string;
  requiresBackgroundCheck?: boolean;
  minimumAge?: number;
  skillsNeeded?: string[];
  remoteOk?: boolean;

}

// ============================================================================
// Website Types
// ============================================================================

export type WebsiteStatus = "pending_review" | "approved" | "rejected" | "suspended";

export interface Website {
  id: string;
  url?: string;
  domain: string;
  status: WebsiteStatus;
  submittedBy?: string;
  submitterType?: string;
  lastScrapedAt?: string;
  snapshotsCount?: number;
  listingsCount?: number;
  listings?: Post[];
  createdAt: string;
}

export interface WebsiteAssessment {
  id: string;
  websiteId: string;
  assessmentMarkdown?: string;
  recommendation?: string;
  confidenceScore?: number;
  organizationName?: string;
  foundedYear?: number;
  generatedAt?: string;
  modelUsed?: string;
  reviewedByHuman?: boolean;
}

export interface WebsiteSearchResult {
  websiteId: string;
  assessmentId: string;
  websiteDomain: string;
  organizationName?: string;
  recommendation?: string;
  assessmentMarkdown?: string;
  similarity: number;
}

// ============================================================================
// Resource Types
// ============================================================================

export type ResourceStatus = "PENDING" | "APPROVED" | "REJECTED";

export interface ResourceContact {
  id: string;
  contactType: string;
  contactValue: string;
  contactLabel?: string;
  isPublic?: boolean;
}

export interface ResourceVersion {
  id: string;
  title: string;
  content: string;
  location?: string;
  changeReason?: string;
  createdAt: string;
}

export interface Resource {
  id: string;
  websiteId?: string;
  title: string;
  content: string;
  location?: string;
  status: ResourceStatus;
  resourceType?: string;
  hasEmbedding?: boolean;
  sourceUrl?: string;
  sourceUrls?: string[];
  contacts?: ResourceContact[];
  tags?: Tag[];
  versions?: ResourceVersion[];
  createdAt: string;
  updatedAt?: string;
}

// ============================================================================
// Chat Types
// ============================================================================

export interface ChatContainer {
  id: string;
  language?: string;
  createdAt: string;
  lastActivityAt?: string;
}

export interface ChatMessage {
  id: string;
  containerId: string;
  role: string;
  content: string;
  authorId?: string;
  moderationStatus?: string;
  parentMessageId?: string;
  sequenceNumber?: number;
  createdAt: string;
  updatedAt?: string;
  editedAt?: string;
}

// ============================================================================
// Job Types
// ============================================================================

export interface JobResult {
  jobId: string;
  status: string;
  message?: string;
}

// ============================================================================
// Query Response Types
// ============================================================================

export interface GetPublishedPostsResult {
  publishedPosts: Post[];
}

export interface GetPostsResult {
  listings: PaginatedResult<Post>;
}

export interface GetPostDetailResult {
  listing: Post | null;
}

export interface GetWebsitesResult {
  websites: PaginatedResult<Website>;
}

export interface GetWebsiteDetailResult {
  website: Website | null;
}

export interface GetWebsiteAssessmentResult {
  websiteAssessment: WebsiteAssessment | null;
}

export interface SearchWebsitesResult {
  searchWebsites: WebsiteSearchResult[];
}

export interface GetResourcesResult {
  resources: PaginatedResult<Resource>;
}

export interface GetResourceResult {
  resource: Resource | null;
}

export interface GetPendingResourcesResult {
  pendingResources: Resource[];
}

export interface GetMessagesResult {
  messages: ChatMessage[];
}

export interface GetContainerResult {
  container: ChatContainer | null;
}

export interface GetRecentChatsResult {
  recentChats: ChatContainer[];
}

export interface ScrapedPostsStatsResult {
  scrapedPendingServices: { totalCount: number };
  scrapedPendingOpportunities: { totalCount: number };
  scrapedPendingBusinesses: { totalCount: number };
}

export interface PendingPostsStatsResult {
  allPending: { totalCount: number };
  pendingServices: { totalCount: number };
  pendingOpportunities: { totalCount: number };
  pendingBusinesses: { totalCount: number };
  pendingUserSubmitted: { totalCount: number };
  pendingScraped: { totalCount: number };
}

// ============================================================================
// Mutation Input Types
// ============================================================================

export interface SubmitResourceLinkInput {
  url: string;
  context?: string;
  submitterContact?: string;
}

export interface EditPostInput {
  title?: string;
  description?: string;
  summary?: string;
  location?: string;
  category?: string;
  urgency?: Urgency;
}

export interface EditResourceInput {
  title?: string;
  content?: string;
  location?: string;
}

export interface TagInput {
  kind: string;
  value: string;
  displayName?: string;
}

// ============================================================================
// Mutation Response Types
// ============================================================================

export interface SendVerificationCodeResult {
  sendVerificationCode: boolean;
}

export interface VerifyCodeResult {
  verifyCode: string;
}

export interface SubmitResourceLinkResult {
  submitResourceLink: {
    jobId: string;
    status: string;
    message?: string;
  };
}

export interface ApprovePostResult {
  approveListing: Post;
}

export interface EditAndApprovePostResult {
  editAndApproveListing: Post;
}

export interface ApproveWebsiteResult {
  approveWebsite: Website;
}

export interface ApproveResourceResult {
  approveResource: Resource;
}

export interface EditResourceResult {
  editResource: Resource;
}

export interface CrawlWebsiteResult {
  crawlWebsite: JobResult;
}

export interface CreateChatResult {
  createChat: ChatContainer;
}

export interface SendMessageResult {
  sendMessage: ChatMessage;
}
