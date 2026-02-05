//! GraphQL query definitions
//!
//! These mirror the queries from web-next/lib/graphql/queries.ts

// ============================================================================
// PUBLIC QUERIES
// ============================================================================

pub const GET_PUBLISHED_POSTS: &str = r#"
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
"#;

pub const SEARCH_ORGANIZATIONS: &str = r#"
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
"#;

// ============================================================================
// ADMIN QUERIES
// ============================================================================

pub const GET_PENDING_POSTS: &str = r#"
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
      }
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
"#;

pub const GET_POST: &str = r#"
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
"#;

pub const GET_ALL_WEBSITES: &str = r#"
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
"#;

pub const GET_WEBSITE_WITH_SNAPSHOTS: &str = r#"
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
      listings {
        id
        title
        status
        createdAt
        sourceUrl
      }
    }
  }
"#;

pub const GET_ORGANIZATIONS: &str = r#"
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
"#;

pub const GET_RESOURCES: &str = r#"
  query GetResources($first: Int, $after: String, $status: ResourceStatusData) {
    resources(first: $first, after: $after, status: $status) {
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
      pageInfo {
        hasNextPage
        endCursor
      }
      totalCount
    }
  }
"#;

pub const GET_ADMIN_STATS: &str = r#"
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
"#;

// ============================================================================
// CHAT QUERIES
// ============================================================================

pub const GET_MESSAGES: &str = r#"
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
"#;

pub const GET_RECENT_CHATS: &str = r#"
  query GetRecentChats($limit: Int) {
    recentChats(limit: $limit) {
      id
      containerType
      language
      createdAt
      lastActivityAt
    }
  }
"#;
