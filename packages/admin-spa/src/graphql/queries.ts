import { gql } from '@apollo/client';

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
