import { gql } from '@apollo/client';

// Public queries
export const GET_PUBLISHED_POSTS = gql`
  query GetPublishedPosts($limit: Int) {
    publishedPosts(limit: $limit) {
      id
      listingId
      status
      publishedAt
      expiresAt
      customTitle
      customDescription
      customTldr
      listing {
        id
        organizationName
        title
        tldr
        description
        location
        urgency
        createdAt
      }
    }
  }
`;

// Admin queries
export const GET_PENDING_POSTS = gql`
  query GetPendingPosts($limit: Int, $offset: Int) {
    listings(status: PENDING_APPROVAL, limit: $limit, offset: $offset) {
      nodes {
        id
        organizationName
        title
        tldr
        description
        urgency
        location
        submissionType
        createdAt
      }
      totalCount
      hasNextPage
    }
  }
`;

export const GET_SCRAPED_PENDING_POSTS = gql`
  query GetScrapedPendingPosts($limit: Int, $offset: Int, $listingType: String) {
    listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      listingType: $listingType
      limit: $limit
      offset: $offset
    ) {
      nodes {
        id
        listingType
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
      totalCount
      hasNextPage
    }
  }
`;

export const GET_ACTIVE_POSTS = gql`
  query GetActivePosts($limit: Int, $offset: Int) {
    listings(status: ACTIVE, limit: $limit, offset: $offset) {
      nodes {
        id
        organizationName
        title
        tldr
        location
        submissionType
        createdAt
      }
      totalCount
      hasNextPage
    }
  }
`;

export const GET_POST_DETAIL = gql`
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

export const GET_WEBSITES = gql`
  query GetWebsites {
    websites {
      id
      domain
      lastScrapedAt
      scrapeFrequencyHours
      active
      status
      createdAt
    }
  }
`;

export const GET_ORGANIZATION_SOURCE_POSTS = gql`
  query GetOrganizationSourcePosts($status: ListingStatusData) {
    listings(status: $status, limit: 1000) {
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
      totalCount
    }
  }
`;

export const GET_POSTS_FOR_LISTING = gql`
  query GetPostsForListing($listingId: Uuid!) {
    postsForListing(listingId: $listingId) {
      id
      status
      expiresAt
      createdAt
    }
  }
`;

export const GET_ORGANIZATION = gql`
  query GetOrganization($id: String!) {
    organization(id: $id) {
      id
      name
      description
      verified
      contactInfo {
        email
        phone
        website
      }
      location
      createdAt
      updatedAt
      businessInfo {
        proceedsPercentage
        proceedsBeneficiaryId
        donationLink
        giftCardLink
        onlineStoreUrl
        isCauseDriven
      }
      tags {
        id
        kind
        value
      }
    }
  }
`;

export const GET_CAUSE_DRIVEN_BUSINESSES = gql`
  query GetCauseDrivenBusinesses {
    organizations(limit: 100) {
      id
      name
      description
      businessInfo {
        proceedsPercentage
        onlineStoreUrl
        isCauseDriven
      }
      tags {
        kind
        value
      }
    }
  }
`;

export const GET_SCRAPED_POSTS_STATS = gql`
  query GetScrapedPostsStats {
    scrapedPendingServices: listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      listingType: "service"
      limit: 1
    ) {
      totalCount
    }
    scrapedPendingOpportunities: listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      listingType: "opportunity"
      limit: 1
    ) {
      totalCount
    }
    scrapedPendingBusinesses: listings(
      status: PENDING_APPROVAL
      submissionType: SCRAPED
      listingType: "business"
      limit: 1
    ) {
      totalCount
    }
  }
`;

// Website management queries
export const GET_PENDING_WEBSITES = gql`
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

export const GET_ALL_WEBSITES = gql`
  query GetAllWebsites($status: String) {
    websites(status: $status) {
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
  }
`;

export const GET_WEBSITE_DETAIL = gql`
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

// Website mutations
export const APPROVE_WEBSITE = gql`
  mutation ApproveWebsite($websiteId: String!) {
    approveWebsite(websiteId: $websiteId) {
      id
      status
    }
  }
`;

export const REJECT_WEBSITE = gql`
  mutation RejectWebsite($websiteId: String!, $reason: String!) {
    rejectWebsite(websiteId: $websiteId, reason: $reason) {
      id
      status
    }
  }
`;

export const SUSPEND_WEBSITE = gql`
  mutation SuspendWebsite($websiteId: String!, $reason: String!) {
    suspendWebsite(websiteId: $websiteId, reason: $reason) {
      id
      status
    }
  }
`;

// Enhanced website detail with snapshot -> listing traceability
export const GET_WEBSITE_WITH_SNAPSHOT_DETAILS = gql`
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

export const GET_ADMIN_STATS = gql`
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

export const GET_WEBSITE_ASSESSMENT = gql`
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

export const SEARCH_WEBSITES = gql`
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

// Chat queries
export const GET_CONTAINER = gql`
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

export const GET_MESSAGES = gql`
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

export const GET_RECENT_CHATS = gql`
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

// Resource queries
export const GET_PENDING_RESOURCES = gql`
  query GetPendingResources {
    pendingResources {
      id
      websiteId
      title
      content
      location
      status
      organizationName
      hasEmbedding
      createdAt
      updatedAt
      sourceUrls
      tags {
        id
        kind
        value
        displayName
      }
    }
  }
`;

export const GET_RESOURCES = gql`
  query GetResources($status: ResourceStatusData, $limit: Int, $offset: Int) {
    resources(status: $status, limit: $limit, offset: $offset) {
      nodes {
        id
        websiteId
        title
        content
        location
        status
        organizationName
        hasEmbedding
        createdAt
        updatedAt
        sourceUrls
        tags {
          id
          kind
          value
          displayName
        }
      }
      totalCount
      hasNextPage
    }
  }
`;

export const GET_RESOURCE = gql`
  query GetResource($id: String!) {
    resource(id: $id) {
      id
      websiteId
      title
      content
      location
      status
      organizationName
      hasEmbedding
      createdAt
      updatedAt
      sourceUrls
      contacts {
        id
        contactType
        contactValue
        contactLabel
        isPublic
      }
      tags {
        id
        kind
        value
        displayName
      }
      versions {
        id
        title
        content
        location
        changeReason
        createdAt
      }
    }
  }
`;
