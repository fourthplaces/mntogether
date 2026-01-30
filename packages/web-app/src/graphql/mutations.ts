import { gql } from '@apollo/client';

// Public mutations
export const SUBMIT_LISTING = gql`
  mutation SubmitListing($input: SubmitListingInput!) {
    submitListing(input: $input) {
      id
      status
    }
  }
`;

export const TRACK_POST_VIEW = gql`
  mutation TrackPostView($postId: ID!) {
    postViewed(postId: $postId)
  }
`;

export const TRACK_POST_CLICK = gql`
  mutation TrackPostClick($postId: ID!) {
    postClicked(postId: $postId)
  }
`;

export const SUBMIT_RESOURCE_LINK = gql`
  mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
    submitResourceLink(input: $input) {
      jobId
      status
      message
    }
  }
`;

// Admin mutations
export const APPROVE_LISTING = gql`
  mutation ApproveListing($listingId: Uuid!) {
    approveListing(listingId: $listingId) {
      id
      status
    }
  }
`;

export const EDIT_AND_APPROVE_LISTING = gql`
  mutation EditAndApproveListing($listingId: Uuid!, $input: EditListingInput!) {
    editAndApproveListing(listingId: $listingId, input: $input) {
      id
      title
      description
      tldr
      status
    }
  }
`;

export const REJECT_LISTING = gql`
  mutation RejectListing($listingId: Uuid!, $reason: String!) {
    rejectListing(listingId: $listingId, reason: $reason)
  }
`;

export const SEND_VERIFICATION_CODE = gql`
  mutation SendVerificationCode($phoneNumber: String!) {
    sendVerificationCode(phoneNumber: $phoneNumber)
  }
`;

export const VERIFY_CODE = gql`
  mutation VerifyCode($phoneNumber: String!, $code: String!) {
    verifyCode(phoneNumber: $phoneNumber, code: $code)
  }
`;

export const SCRAPE_ORGANIZATION = gql`
  mutation ScrapeOrganization($sourceId: Uuid!) {
    scrapeOrganization(sourceId: $sourceId) {
      jobId
      status
      message
    }
  }
`;

export const EXPIRE_POST = gql`
  mutation ExpirePost($postId: Uuid!) {
    expirePost(postId: $postId) {
      id
      status
      expiresAt
    }
  }
`;

export const ARCHIVE_POST = gql`
  mutation ArchivePost($postId: Uuid!) {
    archivePost(postId: $postId) {
      id
      status
    }
  }
`;

export const DELETE_LISTING = gql`
  mutation DeleteListing($listingId: Uuid!) {
    deleteListing(listingId: $listingId)
  }
`;

export const ADD_ORGANIZATION_SCRAPE_URL = gql`
  mutation AddOrganizationScrapeUrl($sourceId: Uuid!, $url: String!) {
    addOrganizationScrapeUrl(sourceId: $sourceId, url: $url)
  }
`;

export const REMOVE_ORGANIZATION_SCRAPE_URL = gql`
  mutation RemoveOrganizationScrapeUrl($sourceId: Uuid!, $url: String!) {
    removeOrganizationScrapeUrl(sourceId: $sourceId, url: $url)
  }
`;

// Agent mutations
export const CREATE_AGENT = gql`
  mutation CreateAgent($input: CreateAgentInput!) {
    createAgent(input: $input) {
      id
      name
      queryTemplate
      description
      enabled
      locationContext
      extractionInstructions
      systemPrompt
      autoApproveDomains
      autoScrape
      autoCreateListings
    }
  }
`;

export const UPDATE_AGENT = gql`
  mutation UpdateAgent($id: ID!, $input: UpdateAgentInput!) {
    updateAgent(id: $id, input: $input) {
      id
      name
      queryTemplate
      description
      enabled
      extractionInstructions
      systemPrompt
      autoApproveDomains
      autoScrape
      autoCreateListings
    }
  }
`;

export const DELETE_AGENT = gql`
  mutation DeleteAgent($id: ID!) {
    deleteAgent(id: $id)
  }
`;

export const TRIGGER_AGENT_SEARCH = gql`
  mutation TriggerAgentSearch($agentId: ID!) {
    triggerAgentSearch(agentId: $agentId) {
      jobId
      status
      message
    }
  }
`;

export const GENERATE_AGENT_CONFIG = gql`
  mutation GenerateAgentConfig($description: String!, $locationContext: String!) {
    generateAgentConfig(description: $description, locationContext: $locationContext) {
      name
      queryTemplate
      extractionInstructions
      systemPrompt
    }
  }
`;

// Domain mutations (if not already defined)
export const APPROVE_DOMAIN = gql`
  mutation ApproveDomain($domainId: String!) {
    approveDomain(domainId: $domainId) {
      id
      status
    }
  }
`;

export const REJECT_DOMAIN = gql`
  mutation RejectDomain($domainId: String!, $reason: String!) {
    rejectDomain(domainId: $domainId, reason: $reason) {
      id
      status
      rejectionReason
    }
  }
`;

export const SUSPEND_DOMAIN = gql`
  mutation SuspendDomain($domainId: String!, $reason: String!) {
    suspendDomain(domainId: $domainId, reason: $reason) {
      id
      status
      rejectionReason
    }
  }
`;

export const REFRESH_PAGE_SNAPSHOT = gql`
  mutation RefreshPageSnapshot($snapshotId: ID!) {
    refreshPageSnapshot(snapshotId: $snapshotId) {
      jobId
      status
      message
    }
  }
`;
