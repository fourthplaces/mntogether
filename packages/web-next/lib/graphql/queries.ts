// GraphQL Queries for web-next
// These are plain strings (no gql tag needed) for use with fetch-based client

// ============================================================================
// PUBLIC QUERIES
// ============================================================================

export const GET_PUBLISHED_POSTS = `
  query GetPublishedPosts($limit: Int) {
    publishedPosts(limit: $limit) {
      id
      organizationName
      title
      tldr
      description
      postType
      category
      capacityStatus
      urgency
      status
      location
      sourceUrl
      createdAt
      updatedAt
    }
  }
`;

// ============================================================================
// ADMIN QUERIES
// ============================================================================

export const GET_PENDING_POSTS = `
  query GetPendingPosts($first: Int, $after: String, $postType: String, $submissionType: SubmissionTypeData) {
    listings(
      status: PENDING_APPROVAL
      first: $first
      after: $after
      postType: $postType
      submissionType: $submissionType
    ) {
      nodes {
        id
        postType
        organizationName
        title
        tldr
        description
        urgency
        location
        category
        sourceUrl
        submissionType
        createdAt

        ... on ServiceListing {
          requiresIdentification
          requiresAppointment
          walkInsAccepted
          remoteAvailable
          inPersonAvailable
          homeVisitsAvailable
          wheelchairAccessible
          interpretationAvailable
          freeService
          slidingScaleFees
          acceptsInsurance
          eveningHours
          weekendHours
        }

        ... on OpportunityListing {
          opportunityType
          timeCommitment
          requiresBackgroundCheck
          minimumAge
          skillsNeeded
          remoteOk
        }

        ... on BusinessListing {
          businessInfo {
            proceedsPercentage
            proceedsBeneficiary {
              id
              name
            }
            donationLink
            giftCardLink
            onlineStoreUrl
          }
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_SCRAPED_PENDING_POSTS = `
  query GetScrapedPendingPosts($first: Int, $after: String, $postType: String) {
    listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      postType: $postType
      first: $first
      after: $after
    ) {
      nodes {
        id
        postType
        organizationName
        title
        tldr
        description
        urgency
        location
        category
        sourceUrl
        submissionType
        createdAt

        ... on ServiceListing {
          requiresIdentification
          requiresAppointment
          walkInsAccepted
          remoteAvailable
          inPersonAvailable
          homeVisitsAvailable
          wheelchairAccessible
          interpretationAvailable
          freeService
          slidingScaleFees
          acceptsInsurance
          eveningHours
          weekendHours
        }

        ... on OpportunityListing {
          opportunityType
          timeCommitment
          requiresBackgroundCheck
          minimumAge
          skillsNeeded
          remoteOk
        }

        ... on BusinessListing {
          businessInfo {
            proceedsPercentage
            proceedsBeneficiary {
              id
              name
            }
            donationLink
            giftCardLink
            onlineStoreUrl
          }
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_ACTIVE_POSTS = `
  query GetActivePosts($first: Int, $after: String) {
    listings(status: ACTIVE, first: $first, after: $after) {
      nodes {
        id
        organizationName
        title
        tldr
        location
        submissionType
        createdAt
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_POST_DETAIL = `
  query GetPostDetail($id: Uuid!) {
    listing(id: $id) {
      id
      organizationName
      title
      tldr
      description
      descriptionMarkdown
      urgency
      location
      status
      submissionType
      createdAt
    }
  }
`;

export const GET_WEBSITES = `
  query GetWebsites($first: Int, $after: String) {
    websites(first: $first, after: $after) {
      nodes {
        id
        url
        status
        createdAt
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_ORGANIZATION_SOURCE_POSTS = `
  query GetOrganizationSourcePosts($first: Int, $after: String, $status: ListingStatusData) {
    listings(first: $first, after: $after, status: $status) {
      nodes {
        id
        organizationName
        title
        tldr
        description
        status
        submissionType
        sourceUrl
        createdAt
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_POSTS_FOR_LISTING = `
  query GetPostsForListing($listingId: Uuid!) {
    postsForListing(listingId: $listingId) {
      id
      status
      expiresAt
      createdAt
    }
  }
`;

export const GET_SCRAPED_POSTS_STATS = `
  query GetScrapedPostsStats {
    scrapedPendingServices: listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      postType: "service"
      limit: 1
    ) {
      totalCount
    }
    scrapedPendingOpportunities: listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      postType: "opportunity"
      limit: 1
    ) {
      totalCount
    }
    scrapedPendingBusinesses: listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      postType: "business"
      limit: 1
    ) {
      totalCount
    }
  }
`;

export const GET_PENDING_POSTS_STATS = `
  query GetPendingPostsStats {
    allPending: listings(status: PENDING_APPROVAL, limit: 1) {
      totalCount
    }
    pendingServices: listings(status: PENDING_APPROVAL, postType: "service", limit: 1) {
      totalCount
    }
    pendingOpportunities: listings(status: PENDING_APPROVAL, postType: "opportunity", limit: 1) {
      totalCount
    }
    pendingBusinesses: listings(status: PENDING_APPROVAL, postType: "business", limit: 1) {
      totalCount
    }
    pendingUserSubmitted: listings(status: PENDING_APPROVAL, submissionType: USER_SUBMITTED, limit: 1) {
      totalCount
    }
    pendingScraped: listings(status: PENDING_APPROVAL, submissionType: SCRAPED, limit: 1) {
      totalCount
    }
  }
`;

// ============================================================================
// WEBSITE MANAGEMENT QUERIES
// ============================================================================

export const GET_PENDING_WEBSITES = `
  query GetPendingWebsites {
    pendingWebsites {
      id
      domain
      status
      submittedBy
      submitterType
      createdAt
    }
  }
`;

export const GET_ALL_WEBSITES = `
  query GetAllWebsites($first: Int, $after: String, $status: String) {
    websites(first: $first, after: $after, status: $status) {
      nodes {
        id
        domain
        status
        lastScrapedAt
        submittedBy
        submitterType
        createdAt
        snapshotsCount
        listingsCount
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
`;

export const GET_WEBSITE_DETAIL = `
  query GetWebsiteDetail($id: Uuid!) {
    website(id: $id) {
      id
      domain
      status
      submittedBy
      submitterType
      createdAt
      snapshotsCount
      listingsCount
    }
  }
`;

export const GET_WEBSITE_WITH_SNAPSHOT_DETAILS = `
  query GetWebsiteWithSnapshotDetails($id: Uuid!) {
    website(id: $id) {
      id
      domain
      status
      submittedBy
      submitterType
      createdAt
      snapshotsCount
      listingsCount
      listings {
        id
        title
        status
        createdAt
      }
    }
  }
`;

export const GET_ADMIN_STATS = `
  query GetAdminStats {
    websites(status: null) {
      id
      status
      listingsCount
      createdAt
    }

    listings {
      id
      status
      createdAt
    }
  }
`;

export const GET_WEBSITE_ASSESSMENT = `
  query GetWebsiteAssessment($websiteId: String!) {
    websiteAssessment(websiteId: $websiteId) {
      id
      websiteId
      assessmentMarkdown
      recommendation
      confidenceScore
      organizationName
      foundedYear
      generatedAt
      modelUsed
      reviewedByHuman
    }
  }
`;

export const SEARCH_WEBSITES = `
  query SearchWebsites($query: String!, $limit: Int, $threshold: Float) {
    searchWebsites(query: $query, limit: $limit, threshold: $threshold) {
      websiteId
      assessmentId
      websiteDomain
      organizationName
      recommendation
      assessmentMarkdown
      similarity
    }
  }
`;

// ============================================================================
// CHAT QUERIES
// ============================================================================

export const GET_CONTAINER = `
  query GetContainer($id: String!) {
    container(id: $id) {
      id
      containerType
      language
      createdAt
      lastActivityAt
    }
  }
`;

export const GET_MESSAGES = `
  query GetMessages($containerId: String!) {
    messages(containerId: $containerId) {
      id
      containerId
      role
      content
      authorId
      moderationStatus
      parentMessageId
      sequenceNumber
      createdAt
      updatedAt
      editedAt
    }
  }
`;

export const GET_RECENT_CHATS = `
  query GetRecentChats($limit: Int) {
    recentChats(limit: $limit) {
      id
      containerType
      language
      createdAt
      lastActivityAt
    }
  }
`;

// ============================================================================
// DETAIL QUERIES FOR ADMIN PAGES
// ============================================================================

export const GET_POST = `
  query GetPost($id: Uuid!) {
    listing(id: $id) {
      id
      organizationName
      title
      tldr
      description
      descriptionMarkdown
      postType
      category
      urgency
      location
      status
      sourceUrl
      websiteId
      createdAt
      tags {
        id
        kind
        value
        displayName
      }
    }
  }
`;

export const GET_WEBSITE_WITH_SNAPSHOTS = `
  query GetWebsiteWithSnapshots($id: Uuid!) {
    website(id: $id) {
      id
      domain
      status
      submittedBy
      submitterType
      lastScrapedAt
      snapshotsCount
      listingsCount
      createdAt
      crawlStatus
      crawlAttemptCount
      maxCrawlRetries
      lastCrawlStartedAt
      lastCrawlCompletedAt
      pagesCrawledCount
      maxPagesPerCrawl
      snapshots {
        url
        siteUrl
        title
        content
        fetchedAt
        listingsCount
      }
      listings {
        id
        title
        status
        createdAt
        sourceUrl
        tags {
          id
          kind
          value
          displayName
        }
      }
    }
  }
`;
