import { graphql } from "@/gql";

export const WebsitesListQuery = graphql(`
  query WebsitesList(
    $status: String
    $search: String
    $limit: Int
    $offset: Int
  ) {
    websites(status: $status, search: $search, limit: $limit, offset: $offset) {
      websites {
        id
        domain
        status
        organizationId
        postCount
        lastCrawledAt
      }
      totalCount
      hasNextPage
    }
  }
`);

export const WebsiteDetailQuery = graphql(`
  query WebsiteDetail($id: ID!) {
    website(id: $id) {
      id
      domain
      status
      active
      organizationId
      postCount
      lastCrawledAt
      createdAt
    }
  }
`);

export const WebsiteDetailFullQuery = graphql(`
  query WebsiteDetailFull($id: ID!) {
    website(id: $id) {
      id
      domain
      status
      active
      organizationId
      postCount
      lastCrawledAt
      createdAt
      posts(limit: 100) {
        posts {
          id
          title
          summary
          status
          postType
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
      pages(limit: 50) {
        url
        content
      }
      pageCount
      assessment {
        id
        websiteId
        assessmentMarkdown
        confidenceScore
      }
      organization {
        id
        name
      }
    }
    organizations {
      id
      name
      description
      websiteCount
      socialProfileCount
    }
  }
`);

export const WebsitePagesQuery = graphql(`
  query WebsitePages($domain: String!, $limit: Int) {
    websitePages(domain: $domain, limit: $limit) {
      url
      content
    }
  }
`);

export const WebsitePageCountQuery = graphql(`
  query WebsitePageCount($domain: String!) {
    websitePageCount(domain: $domain)
  }
`);

export const WebsiteAssessmentQuery = graphql(`
  query WebsiteAssessment($websiteId: ID!) {
    websiteAssessment(websiteId: $websiteId) {
      id
      websiteId
      assessmentMarkdown
      confidenceScore
    }
  }
`);

export const WebsitePostsQuery = graphql(`
  query WebsitePosts($websiteId: ID!, $limit: Int) {
    websitePosts(websiteId: $websiteId, limit: $limit) {
      posts {
        id
        title
        summary
        status
        postType
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
`);

export const SubmitNewWebsiteMutation = graphql(`
  mutation SubmitNewWebsite($url: String!) {
    submitNewWebsite(url: $url) {
      id
      domain
    }
  }
`);

export const ApproveWebsiteMutation = graphql(`
  mutation ApproveWebsite($id: ID!) {
    approveWebsite(id: $id) {
      id
      status
    }
  }
`);

export const RejectWebsiteMutation = graphql(`
  mutation RejectWebsite($id: ID!, $reason: String!) {
    rejectWebsite(id: $id, reason: $reason) {
      id
      status
    }
  }
`);

export const CrawlWebsiteMutation = graphql(`
  mutation CrawlWebsite($id: ID!) {
    crawlWebsite(id: $id)
  }
`);

export const GenerateWebsiteAssessmentMutation = graphql(`
  mutation GenerateWebsiteAssessment($id: ID!) {
    generateWebsiteAssessment(id: $id)
  }
`);

export const RegenerateWebsitePostsMutation = graphql(`
  mutation RegenerateWebsitePosts($id: ID!) {
    regenerateWebsitePosts(id: $id) {
      workflowId
      status
    }
  }
`);

export const DeduplicateWebsitePostsMutation = graphql(`
  mutation DeduplicateWebsitePosts($id: ID!) {
    deduplicateWebsitePosts(id: $id) {
      workflowId
      status
    }
  }
`);

export const ExtractWebsiteOrganizationMutation = graphql(`
  mutation ExtractWebsiteOrganization($id: ID!) {
    extractWebsiteOrganization(id: $id) {
      id
      organizationId
    }
  }
`);

export const AssignWebsiteOrganizationMutation = graphql(`
  mutation AssignWebsiteOrganization($id: ID!, $organizationId: ID!) {
    assignWebsiteOrganization(id: $id, organizationId: $organizationId) {
      id
      organizationId
    }
  }
`);

export const UnassignWebsiteOrganizationMutation = graphql(`
  mutation UnassignWebsiteOrganization($id: ID!) {
    unassignWebsiteOrganization(id: $id) {
      id
      organizationId
    }
  }
`);

export const ApprovePostInlineMutation = graphql(`
  mutation ApprovePostInline($id: ID!) {
    approvePostInline(id: $id) {
      id
      status
    }
  }
`);

export const RejectPostInlineMutation = graphql(`
  mutation RejectPostInline($id: ID!, $reason: String) {
    rejectPostInline(id: $id, reason: $reason) {
      id
      status
    }
  }
`);
