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
      autoApproveWebsites
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
      autoApproveWebsites
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

export const REFRESH_PAGE_SNAPSHOT = gql`
  mutation RefreshPageSnapshot($snapshotId: String!) {
    refreshPageSnapshot(snapshotId: $snapshotId) {
      jobId
      status
      message
    }
  }
`;

export const GENERATE_WEBSITE_ASSESSMENT = gql`
  mutation GenerateWebsiteAssessment($websiteId: String!) {
    generateWebsiteAssessment(websiteId: $websiteId)
  }
`;

export const CRAWL_WEBSITE = gql`
  mutation CrawlWebsite($websiteId: Uuid!) {
    crawlWebsite(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

// Chat mutations
export const CREATE_CHAT = gql`
  mutation CreateChat($language: String) {
    createChat(language: $language) {
      id
      containerType
      language
      createdAt
      lastActivityAt
    }
  }
`;

export const SEND_MESSAGE = gql`
  mutation SendMessage($containerId: String!, $content: String!) {
    sendMessage(containerId: $containerId, content: $content) {
      id
      containerId
      role
      content
      authorId
      createdAt
    }
  }
`;

export const SIGNAL_TYPING = gql`
  mutation SignalTyping($containerId: String!) {
    signalTyping(containerId: $containerId)
  }
`;
