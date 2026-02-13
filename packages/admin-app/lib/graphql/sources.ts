import { graphql } from "@/gql";
import "./fragments";

export const SourcesListQuery = graphql(`
  query SourcesList(
    $status: String
    $sourceType: String
    $search: String
    $limit: Int
    $offset: Int
  ) {
    sources(
      status: $status
      sourceType: $sourceType
      search: $search
      limit: $limit
      offset: $offset
    ) {
      sources {
        id
        sourceType
        identifier
        url
        status
        organizationName
        postCount
        lastScrapedAt
      }
      totalCount
      hasNextPage
      hasPreviousPage
    }
  }
`);

export const SearchSourcesByContentQuery = graphql(`
  query SearchSourcesByContent($query: String!, $limit: Int) {
    searchSourcesByContent(query: $query, limit: $limit) {
      sources {
        id
        sourceType
        identifier
        url
        status
        organizationName
        postCount
        lastScrapedAt
      }
      totalCount
      hasNextPage
      hasPreviousPage
    }
  }
`);

export const SourceDetailQuery = graphql(`
  query SourceDetail($id: ID!) {
    source(id: $id) {
      ...SourceFields
    }
  }
`);

export const SourceDetailFullQuery = graphql(`
  query SourceDetailFull($id: ID!) {
    source(id: $id) {
      ...SourceFields
      pages {
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
      newsletterSource {
        id
        sourceId
        ingestEmail
        signupFormUrl
        subscriptionStatus
        confirmationLink
        expectedSenderDomain
        lastNewsletterReceivedAt
        newslettersReceivedCount
      }
      detectedNewsletterForms {
        id
        websiteSourceId
        formUrl
        formType
        requiresExtraFields
        extraFieldsDetected
        status
      }
    }
    organizations {
      id
      name
      description
    }
  }
`);

export const SourcePagesQuery = graphql(`
  query SourcePages($sourceId: ID!) {
    sourcePages(sourceId: $sourceId) {
      url
      content
    }
  }
`);

export const SourcePageCountQuery = graphql(`
  query SourcePageCount($sourceId: ID!) {
    sourcePageCount(sourceId: $sourceId)
  }
`);

export const SourceAssessmentQuery = graphql(`
  query SourceAssessment($sourceId: ID!) {
    sourceAssessment(sourceId: $sourceId) {
      id
      websiteId
      assessmentMarkdown
      confidenceScore
    }
  }
`);

export const ExtractionPageQuery = graphql(`
  query ExtractionPage($url: String!) {
    extractionPage(url: $url) {
      url
      content
    }
  }
`);

export const WorkflowStatusQuery = graphql(`
  query WorkflowStatus($workflowName: String!, $workflowId: String!) {
    workflowStatus(workflowName: $workflowName, workflowId: $workflowId)
  }
`);

export const SubmitWebsiteMutation = graphql(`
  mutation SubmitWebsite($url: String!) {
    submitWebsite(url: $url) {
      id
      sourceType
      identifier
    }
  }
`);

export const LightCrawlAllMutation = graphql(`
  mutation LightCrawlAll {
    lightCrawlAll {
      sourcesQueued
    }
  }
`);

export const ApproveSourceMutation = graphql(`
  mutation ApproveSource($id: ID!) {
    approveSource(id: $id) {
      id
      status
    }
  }
`);

export const RejectSourceMutation = graphql(`
  mutation RejectSource($id: ID!, $reason: String!) {
    rejectSource(id: $id, reason: $reason) {
      id
      status
    }
  }
`);

export const CrawlSourceMutation = graphql(`
  mutation CrawlSource($id: ID!) {
    crawlSource(id: $id)
  }
`);

export const GenerateSourceAssessmentMutation = graphql(`
  mutation GenerateSourceAssessment($id: ID!) {
    generateSourceAssessment(id: $id)
  }
`);

export const RegenerateSourcePostsMutation = graphql(`
  mutation RegenerateSourcePosts($id: ID!) {
    regenerateSourcePosts(id: $id) {
      workflowId
      status
    }
  }
`);

export const DeduplicateSourcePostsMutation = graphql(`
  mutation DeduplicateSourcePosts($id: ID!) {
    deduplicateSourcePosts(id: $id) {
      workflowId
      status
    }
  }
`);

export const ExtractSourceOrganizationMutation = graphql(`
  mutation ExtractSourceOrganization($id: ID!) {
    extractSourceOrganization(id: $id) {
      id
      organizationId
    }
  }
`);

export const AssignSourceOrganizationMutation = graphql(`
  mutation AssignSourceOrganization($id: ID!, $organizationId: ID!) {
    assignSourceOrganization(id: $id, organizationId: $organizationId) {
      id
      organizationId
    }
  }
`);

export const UnassignSourceOrganizationMutation = graphql(`
  mutation UnassignSourceOrganization($id: ID!) {
    unassignSourceOrganization(id: $id) {
      id
      organizationId
    }
  }
`);

export const SubscribeNewsletterMutation = graphql(`
  mutation SubscribeNewsletter($formId: ID!, $organizationId: ID) {
    subscribeNewsletter(formId: $formId, organizationId: $organizationId) {
      workflowId
      status
    }
  }
`);

export const ConfirmNewsletterMutation = graphql(`
  mutation ConfirmNewsletter($sourceId: ID!) {
    confirmNewsletter(sourceId: $sourceId) {
      workflowId
      status
    }
  }
`);

export const DeactivateNewsletterMutation = graphql(`
  mutation DeactivateNewsletter($sourceId: ID!) {
    deactivateNewsletter(sourceId: $sourceId) {
      id
      status
    }
  }
`);

export const ReactivateNewsletterMutation = graphql(`
  mutation ReactivateNewsletter($sourceId: ID!) {
    reactivateNewsletter(sourceId: $sourceId) {
      id
      status
    }
  }
`);
