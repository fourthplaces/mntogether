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

export type PostType = "story" | "notice" | "exchange" | "event" | "spotlight" | "reference";
export type PostStatus = "PENDING_APPROVAL" | "ACTIVE" | "REJECTED" | "EXPIRED" | "ARCHIVED";
export interface Post {
  id: string;
  title: string;
  bodyRaw: string;
  postType?: PostType;
  isUrgent: boolean;
  status: PostStatus;
  location?: string;
  weight?: string;
  priority?: number;
  tags?: Tag[];
  createdAt: string;
  updatedAt?: string;
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

export interface PostStatsResult {
  total: number;
  stories: number;
  notices: number;
  exchanges: number;
  events: number;
  spotlights: number;
  references: number;
  userSubmitted: number;
}

// ============================================================================
// Mutation Input Types
// ============================================================================

export interface EditPostInput {
  title?: string;
  bodyRaw?: string;
  location?: string;
  isUrgent?: boolean;
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

