import { gql } from '@apollo/client';

// Public mutations
export const SUBMIT_POST = gql`
  mutation SubmitPost($input: SubmitListingInput!) {
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
export const APPROVE_POST = gql`
  mutation ApprovePost($listingId: Uuid!) {
    approveListing(listingId: $listingId) {
      id
      status
    }
  }
`;

export const EDIT_AND_APPROVE_POST = gql`
  mutation EditAndApprovePost($listingId: Uuid!, $input: EditListingInput!) {
    editAndApproveListing(listingId: $listingId, input: $input) {
      id
      title
      description
      tldr
      status
    }
  }
`;

export const REJECT_POST = gql`
  mutation RejectPost($listingId: Uuid!, $reason: String!) {
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

export const DELETE_POST = gql`
  mutation DeletePost($listingId: Uuid!) {
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

export const REGENERATE_POSTS = gql`
  mutation RegeneratePosts($websiteId: Uuid!) {
    regeneratePosts(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

export const REGENERATE_PAGE_SUMMARIES = gql`
  mutation RegeneratePageSummaries($websiteId: Uuid!) {
    regeneratePageSummaries(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

export const REGENERATE_PAGE_SUMMARY = gql`
  mutation RegeneratePageSummary($pageSnapshotId: Uuid!) {
    regeneratePageSummary(pageSnapshotId: $pageSnapshotId) {
      jobId
      status
      message
    }
  }
`;

export const REGENERATE_PAGE_POSTS = gql`
  mutation RegeneratePagePosts($pageSnapshotId: Uuid!) {
    regeneratePagePosts(pageSnapshotId: $pageSnapshotId) {
      jobId
      status
      message
    }
  }
`;

// Post tag mutations
export const UPDATE_POST_TAGS = gql`
  mutation UpdatePostTags($listingId: Uuid!, $tags: [TagInput!]!) {
    updateListingTags(listingId: $listingId, tags: $tags) {
      id
      tags {
        id
        kind
        value
        displayName
      }
    }
  }
`;

export const ADD_POST_TAG = gql`
  mutation AddPostTag($listingId: Uuid!, $tagKind: String!, $tagValue: String!, $displayName: String) {
    addListingTag(listingId: $listingId, tagKind: $tagKind, tagValue: $tagValue, displayName: $displayName) {
      id
      kind
      value
      displayName
    }
  }
`;

export const REMOVE_POST_TAG = gql`
  mutation RemovePostTag($listingId: Uuid!, $tagId: String!) {
    removeListingTag(listingId: $listingId, tagId: $tagId)
  }
`;

// Chat mutations
export const CREATE_CHAT = gql`
  mutation CreateChat($language: String, $withAgent: String) {
    createChat(language: $language, withAgent: $withAgent) {
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

// Resource mutations
export const APPROVE_RESOURCE = gql`
  mutation ApproveResource($resourceId: String!) {
    approveResource(resourceId: $resourceId) {
      id
      status
    }
  }
`;

export const REJECT_RESOURCE = gql`
  mutation RejectResource($resourceId: String!, $reason: String!) {
    rejectResource(resourceId: $resourceId, reason: $reason) {
      id
      status
    }
  }
`;

export const EDIT_RESOURCE = gql`
  mutation EditResource($resourceId: String!, $input: EditResourceInput!) {
    editResource(resourceId: $resourceId, input: $input) {
      id
      title
      content
      location
      status
    }
  }
`;

export const EDIT_AND_APPROVE_RESOURCE = gql`
  mutation EditAndApproveResource($resourceId: String!, $input: EditResourceInput!) {
    editAndApproveResource(resourceId: $resourceId, input: $input) {
      id
      title
      content
      location
      status
    }
  }
`;

export const DELETE_RESOURCE = gql`
  mutation DeleteResource($resourceId: String!) {
    deleteResource(resourceId: $resourceId)
  }
`;

export const GENERATE_MISSING_EMBEDDINGS = gql`
  mutation GenerateMissingEmbeddings($batchSize: Int) {
    generateMissingEmbeddings(batchSize: $batchSize) {
      processed
      failed
      remaining
    }
  }
`;
