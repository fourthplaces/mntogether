// TypeScript types for GraphQL API responses

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
// Business Info (used by Post)
// ============================================================================

export interface BusinessInfo {
  proceedsPercentage?: number;
  proceedsBeneficiaryId?: string;
  proceedsBeneficiary?: {
    id: string;
    name: string;
  };
  donationLink?: string;
  giftCardLink?: string;
  onlineStoreUrl?: string;
  isCauseDriven?: boolean;
}

// ============================================================================
// Post/Listing Types
// ============================================================================

export type PostType = "service" | "opportunity" | "business" | "professional";
export type ListingStatus = "PENDING_APPROVAL" | "ACTIVE" | "REJECTED" | "EXPIRED" | "ARCHIVED";
export type CapacityStatus = "accepting" | "paused" | "at_capacity";
export type SubmissionType = "SCRAPED" | "MANUAL" | "USER_SUBMITTED";
export type Urgency = "urgent" | "high" | "medium" | "low";

export interface Post {
  id: string;
  organizationName: string;
  title: string;
  tldr?: string;
  description: string;
  descriptionMarkdown?: string;
  postType?: PostType;
  category?: string;
  capacityStatus?: CapacityStatus;
  urgency?: Urgency;
  status: ListingStatus;
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

  // Business-specific fields
  businessInfo?: BusinessInfo;
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
// Chat Types
// ============================================================================

export interface ChatContainer {
  id: string;
  containerType: string;
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

export interface GetListingsResult {
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

export interface EditListingInput {
  title?: string;
  description?: string;
  tldr?: string;
  location?: string;
  category?: string;
  urgency?: Urgency;
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

export interface ApprovePostResult {
  approveListing: Post;
}

export interface EditAndApprovePostResult {
  editAndApproveListing: Post;
}

export interface ApproveWebsiteResult {
  approveWebsite: Website;
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
