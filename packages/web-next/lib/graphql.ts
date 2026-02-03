// GraphQL client for server-side data fetching

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/graphql";

export interface GraphQLResponse<T> {
  data?: T;
  errors?: Array<{ message: string }>;
}

export interface GraphQLFetchOptions {
  revalidate?: number | false;
  cache?: RequestCache;
  token?: string;
}

export async function graphqlFetch<T>(
  query: string,
  variables?: Record<string, any>,
  options?: GraphQLFetchOptions
): Promise<T> {
  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };

  // Add auth token if provided
  if (options?.token) {
    headers["Authorization"] = `Bearer ${options.token}`;
  }

  const response = await fetch(API_URL, {
    method: "POST",
    headers,
    body: JSON.stringify({
      query,
      variables,
    }),
    next: {
      revalidate: options?.revalidate ?? 60, // Default: revalidate every 60 seconds
    },
    cache: options?.cache,
  });

  if (!response.ok) {
    throw new Error(`GraphQL request failed: ${response.statusText}`);
  }

  const json: GraphQLResponse<T> = await response.json();

  if (json.errors) {
    throw new Error(json.errors[0].message);
  }

  if (!json.data) {
    throw new Error("No data returned from GraphQL");
  }

  return json.data;
}

// Client-side GraphQL fetch (for use in client components)
export async function graphqlFetchClient<T>(
  query: string,
  variables?: Record<string, any>,
  token?: string
): Promise<T> {
  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(API_URL, {
    method: "POST",
    headers,
    body: JSON.stringify({
      query,
      variables,
    }),
  });

  if (!response.ok) {
    throw new Error(`GraphQL request failed: ${response.statusText}`);
  }

  const json: GraphQLResponse<T> = await response.json();

  if (json.errors) {
    throw new Error(json.errors[0].message);
  }

  if (!json.data) {
    throw new Error("No data returned from GraphQL");
  }

  return json.data;
}

// ============================================================================
// QUERIES
// ============================================================================

// Organizations
export const SEARCH_ORGANIZATIONS = `
  query SearchOrganizationsSemantic($query: String!, $limit: Int) {
    searchOrganizationsSemantic(query: $query, limit: $limit) {
      organization {
        id
        name
        description
        summary
        website
        phone
        primaryAddress
      }
      similarityScore
    }
  }
`;

export const GET_ORGANIZATIONS = `
  query GetOrganizations($first: Int, $after: String) {
    organizations(first: $first, after: $after) {
      nodes {
        id
        name
        description
        contactInfo {
          website
          phone
        }
        location
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_ORGANIZATION = `
  query GetOrganization($id: String!) {
    organization(id: $id) {
      id
      name
      description
      summary
      website
      phone
      primaryAddress
    }
  }
`;

// Posts
export const GET_PUBLISHED_POSTS = `
  query GetPublishedPosts($limit: Int) {
    publishedPosts(limit: $limit) {
      id
      title
      description
      organizationName
      createdAt
    }
  }
`;

export const GET_POST = `
  query GetPost($id: String!) {
    post(id: $id) {
      id
      title
      description
      organizationName
      createdAt
    }
  }
`;

// Listings (new multi-sided platform queries)
export const GET_LISTING = `
  query GetListing($id: String!) {
    listing(id: $id) {
      id
      organizationId
      listingType
      title
      description
      category
      status
      capacityStatus
      contactInfo
      createdAt
      updatedAt
    }
  }
`;

export const GET_LISTINGS_BY_TYPE = `
  query GetListingsByType($listingType: String!, $limit: Int, $offset: Int) {
    listingsByType(listingType: $listingType, limit: $limit, offset: $offset) {
      id
      organizationId
      listingType
      title
      description
      category
      status
      capacityStatus
      contactInfo
      createdAt
      updatedAt
    }
  }
`;

export const GET_LISTINGS_BY_CATEGORY = `
  query GetListingsByCategory($category: String!, $limit: Int, $offset: Int) {
    listingsByCategory(category: $category, limit: $limit, offset: $offset) {
      id
      organizationId
      listingType
      title
      description
      category
      status
      capacityStatus
      contactInfo
      createdAt
      updatedAt
    }
  }
`;

export const SEARCH_LISTINGS = `
  query SearchListings(
    $listingType: String
    $category: String
    $capacityStatus: String
    $limit: Int
    $offset: Int
  ) {
    searchListings(
      listingType: $listingType
      category: $category
      capacityStatus: $capacityStatus
      limit: $limit
      offset: $offset
    ) {
      id
      organizationId
      listingType
      title
      description
      category
      status
      capacityStatus
      contactInfo
      createdAt
      updatedAt
    }
  }
`;

// ============================================================================
// MUTATIONS
// ============================================================================

// Authentication
export const SEND_VERIFICATION_CODE = `
  mutation SendVerificationCode($phoneNumber: String!) {
    sendVerificationCode(phoneNumber: $phoneNumber)
  }
`;

export const VERIFY_CODE = `
  mutation VerifyCode($phoneNumber: String!, $code: String!) {
    verifyCode(phoneNumber: $phoneNumber, code: $code)
  }
`;

export const LOGOUT = `
  mutation Logout($sessionToken: String!) {
    logout(sessionToken: $sessionToken)
  }
`;

// Public mutations
export const SUBMIT_RESOURCE_LINK = `
  mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
    submitResourceLink(input: $input) {
      success
      message
      organizationId
      sourceId
    }
  }
`;

export const TRACK_POST_VIEW = `
  mutation TrackPostView($postId: String!) {
    postViewed(postId: $postId)
  }
`;

export const TRACK_POST_CLICK = `
  mutation TrackPostClick($postId: String!) {
    postClicked(postId: $postId)
  }
`;

// Listing mutations
export const CREATE_LISTING = `
  mutation CreateListing($input: CreateListingInput!) {
    createListing(input: $input) {
      id
      organizationId
      listingType
      title
      description
      category
      status
      capacityStatus
      contactInfo
      createdAt
      updatedAt
    }
  }
`;

export const UPDATE_LISTING_STATUS = `
  mutation UpdateListingStatus($listingId: String!, $status: String!) {
    updateListingStatus(listingId: $listingId, status: $status) {
      id
      status
    }
  }
`;

export const UPDATE_LISTING_CAPACITY = `
  mutation UpdateListingCapacity($listingId: String!, $capacityStatus: String!) {
    updateListingCapacity(listingId: $listingId, capacityStatus: $capacityStatus) {
      id
      capacityStatus
    }
  }
`;
