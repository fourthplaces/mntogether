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
export const GET_PENDING_LISTINGS = gql`
  query GetPendingListings($limit: Int, $offset: Int) {
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

export const GET_SCRAPED_PENDING_LISTINGS = gql`
  query GetScrapedPendingListings($limit: Int, $offset: Int, $listingType: String) {
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

export const GET_ACTIVE_LISTINGS = gql`
  query GetActiveListings($limit: Int, $offset: Int) {
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

export const GET_LISTING_DETAIL = gql`
  query GetListingDetail($id: Uuid!) {
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

export const GET_ORGANIZATION_SOURCES = gql`
  query GetOrganizationSources {
    organizationSources {
      id
      organizationName
      sourceUrl
      scrapeUrls
      lastScrapedAt
      scrapeFrequencyHours
      active
      createdAt
    }
  }
`;

export const GET_ORGANIZATION_SOURCE_LISTINGS = gql`
  query GetOrganizationSourceListings($status: ListingStatusData) {
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

export const GET_SCRAPED_LISTINGS_STATS = gql`
  query GetScrapedListingsStats {
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

// Domain management queries
export const GET_PENDING_DOMAINS = gql`
  query GetPendingDomains {
    pendingDomains {
      id
      websiteUrl
      status
      submittedBy
      submitterType
      submissionContext
      createdAt
    }
  }
`;

export const GET_ALL_DOMAINS = gql`
  query GetAllDomains($status: String, $agentId: String) {
    domains(status: $status, agentId: $agentId) {
      id
      websiteUrl
      status
      lastScrapedAt
      submittedBy
      submitterType
      agentId
      createdAt
      snapshotsCount
      listingsCount
    }
  }
`;

export const GET_DOMAIN_DETAIL = gql`
  query GetDomainDetail($id: ID!) {
    domain(id: $id) {
      id
      websiteUrl
      status
      submittedBy
      submitterType
      submissionContext
      reviewedBy
      reviewedAt
      rejectionReason
      createdAt
      updatedAt
      snapshots {
        id
        pageUrl
        scrapeStatus
        lastScrapedAt
        scrapeError
        submittedAt
      }
      listings {
        id
        title
        status
        createdAt
      }
    }
  }
`;

// Domain mutations
export const APPROVE_DOMAIN = gql`
  mutation ApproveDomain($websiteId: ID!) {
    approveDomain(websiteId: $websiteId) {
      id
      status
    }
  }
`;

export const REJECT_DOMAIN = gql`
  mutation RejectDomain($websiteId: ID!, $reason: String!) {
    rejectDomain(websiteId: $websiteId, reason: $reason) {
      id
      status
      rejectionReason
    }
  }
`;

export const SUSPEND_DOMAIN = gql`
  mutation SuspendDomain($websiteId: ID!, $reason: String!) {
    suspendDomain(websiteId: $websiteId, reason: $reason) {
      id
      status
      rejectionReason
    }
  }
`;

// Enhanced domain detail with snapshot -> listing traceability
export const GET_DOMAIN_WITH_SNAPSHOT_DETAILS = gql`
  query GetDomainWithSnapshotDetails($id: ID!) {
    domain(id: $id) {
      id
      websiteUrl
      status
      submittedBy
      submitterType
      submissionContext
      reviewedBy
      reviewedAt
      rejectionReason
      createdAt
      updatedAt
      
      snapshots {
        id
        pageUrl
        scrapeStatus
        lastScrapedAt
        scrapeError
        submittedAt
        
        # Show cached page content if available
        pageSnapshot {
          id
          contentHash
          crawledAt
          markdown
        }
        
        # Show listings extracted from this specific page
        listings {
          id
          title
          status
          urgency
          createdAt
          organizationName
        }
      }
      
      # Total listings from all pages in this domain
      totalListings: listings {
        id
        title
        status
        sourceUrl
        createdAt
      }
    }
  }
`;

// Query to see listings by source page
export const GET_LISTINGS_BY_PAGE = gql`
  query GetListingsByPage($websiteId: ID!, $pageUrl: String!) {
    listingsByPage(websiteId: $websiteId, pageUrl: $pageUrl) {
      id
      title
      description
      status
      urgency
      organizationName
      sourceUrl
      createdAt
      extractionConfidence
    }
  }
`;

// Agent queries
export const GET_ALL_AGENTS = gql`
  query GetAllAgents {
    agents {
      id
      name
      queryTemplate
      description
      enabled
      searchFrequencyHours
      lastSearchedAt
      locationContext
      searchDepth
      maxResults
      daysRange
      minRelevanceScore
      extractionInstructions
      systemPrompt
      autoApproveDomains
      autoScrape
      autoCreateListings
      totalSearchesRun
      totalDomainsDiscovered
      totalDomainsApproved
      createdAt
    }
  }
`;

export const GET_AGENT = gql`
  query GetAgent($id: ID!) {
    agent(id: $id) {
      id
      name
      queryTemplate
      description
      enabled
      searchFrequencyHours
      lastSearchedAt
      locationContext
      searchDepth
      maxResults
      daysRange
      minRelevanceScore
      extractionInstructions
      systemPrompt
      autoApproveDomains
      autoScrape
      autoCreateListings
      totalSearchesRun
      totalDomainsDiscovered
      totalDomainsApproved
      createdAt
      updatedAt
    }
  }
`;

export const GET_ADMIN_STATS = gql`
  query GetAdminStats {
    domains(status: null) {
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

export const GET_DOMAIN_ASSESSMENT = gql`
  query GetDomainAssessment($domainId: String!) {
    domainAssessment(domainId: $domainId) {
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
      websiteUrl
      organizationName
      recommendation
      assessmentMarkdown
      similarity
    }
  }
`;
