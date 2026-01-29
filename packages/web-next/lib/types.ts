// TypeScript types for GraphQL API responses

// ============================================================================
// Organization Types
// ============================================================================

export interface Organization {
  id: string;
  name: string;
  description?: string;
  summary?: string;
  website?: string;
  phone?: string;
  primaryAddress?: string;
}

export interface OrganizationMatch {
  organization: Organization;
  similarityScore: number;
}

// ============================================================================
// Post Types
// ============================================================================

export interface Post {
  id: string;
  title: string;
  description: string;
  organizationName: string;
  createdAt: string;
}

// ============================================================================
// Listing Types
// ============================================================================

export type ListingType = "service" | "opportunity" | "business";
export type ListingStatus = "draft" | "active" | "inactive" | "archived";
export type CapacityStatus = "available" | "limited" | "full" | "waitlist";

export interface Listing {
  id: string;
  organizationId: string;
  listingType: ListingType;
  title: string;
  description: string;
  category: string;
  status: ListingStatus;
  capacityStatus: CapacityStatus;
  contactInfo?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateListingInput {
  organizationId: string;
  listingType: ListingType;
  title: string;
  description: string;
  category: string;
  capacityStatus?: CapacityStatus;
  contactInfo?: string;
}

// ============================================================================
// Authentication Types
// ============================================================================

export interface AuthToken {
  token: string;
  expiresAt: string;
}

// ============================================================================
// Query Response Types
// ============================================================================

export interface SearchOrganizationsResult {
  searchOrganizationsSemantic: OrganizationMatch[];
}

export interface GetOrganizationsResult {
  organizations: Organization[];
}

export interface GetOrganizationResult {
  organization: Organization | null;
}

export interface GetPublishedPostsResult {
  publishedPosts: Post[];
}

export interface GetPostResult {
  post: Post | null;
}

export interface GetListingResult {
  listing: Listing | null;
}

export interface GetListingsByTypeResult {
  listingsByType: Listing[];
}

export interface GetListingsByCategoryResult {
  listingsByCategory: Listing[];
}

export interface SearchListingsResult {
  searchListings: Listing[];
}

// ============================================================================
// Mutation Response Types
// ============================================================================

export interface SendVerificationCodeResult {
  sendVerificationCode: boolean;
}

export interface VerifyCodeResult {
  verifyCode: string; // Returns JWT token
}

export interface LogoutResult {
  logout: boolean;
}

export interface SubmitResourceLinkResult {
  submitResourceLink: {
    success: boolean;
    message: string;
    organizationId?: string;
    sourceId?: string;
  };
}

export interface CreateListingResult {
  createListing: Listing;
}

export interface UpdateListingStatusResult {
  updateListingStatus: Listing;
}

export interface UpdateListingCapacityResult {
  updateListingCapacity: Listing;
}
