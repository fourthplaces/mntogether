import { gql } from '@apollo/client';

// Public queries
export const GET_PUBLISHED_POSTS = gql`
  query GetPublishedPosts($limit: Int) {
    publishedPosts(limit: $limit) {
      id
      needId
      status
      publishedAt
      expiresAt
      customTitle
      customDescription
      customTldr
      need {
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
export const GET_PENDING_NEEDS = gql`
  query GetPendingNeeds($limit: Int, $offset: Int) {
    needs(status: PENDING_APPROVAL, limit: $limit, offset: $offset) {
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

export const GET_ACTIVE_NEEDS = gql`
  query GetActiveNeeds($limit: Int, $offset: Int) {
    needs(status: ACTIVE, limit: $limit, offset: $offset) {
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

export const GET_NEED_DETAIL = gql`
  query GetNeedDetail($id: Uuid!) {
    need(id: $id) {
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

export const GET_ORGANIZATION_SOURCE_NEEDS = gql`
  query GetOrganizationSourceNeeds($status: NeedStatusData) {
    needs(status: $status, limit: 1000) {
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

export const GET_POSTS_FOR_NEED = gql`
  query GetPostsForNeed($needId: Uuid!) {
    postsForNeed(needId: $needId) {
      id
      status
      expiresAt
      createdAt
    }
  }
`;
