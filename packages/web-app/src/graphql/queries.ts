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
        contactInfo {
          email
          phone
          website
        }
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
        contactInfo {
          email
          phone
          website
        }
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
        contactInfo {
          email
          phone
          website
        }
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
      contactInfo {
        email
        phone
        website
      }
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
