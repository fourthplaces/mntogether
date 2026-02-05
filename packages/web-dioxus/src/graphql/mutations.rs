//! GraphQL mutation definitions
//!
//! These mirror the mutations from web-next/lib/graphql/mutations.ts

// ============================================================================
// PUBLIC MUTATIONS
// ============================================================================

pub const SUBMIT_RESOURCE_LINK: &str = r#"
  mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
    submitResourceLink(input: $input) {
      jobId
      status
      message
    }
  }
"#;

pub const TRACK_POST_VIEW: &str = r#"
  mutation TrackPostView($postId: ID!) {
    postViewed(postId: $postId)
  }
"#;

pub const TRACK_POST_CLICK: &str = r#"
  mutation TrackPostClick($postId: ID!) {
    postClicked(postId: $postId)
  }
"#;

// ============================================================================
// AUTHENTICATION MUTATIONS
// ============================================================================

pub const SEND_VERIFICATION_CODE: &str = r#"
  mutation SendVerificationCode($phoneNumber: String!) {
    sendVerificationCode(phoneNumber: $phoneNumber)
  }
"#;

pub const VERIFY_CODE: &str = r#"
  mutation VerifyCode($phoneNumber: String!, $code: String!) {
    verifyCode(phoneNumber: $phoneNumber, code: $code)
  }
"#;

// ============================================================================
// ADMIN POST MUTATIONS
// ============================================================================

pub const APPROVE_POST: &str = r#"
  mutation ApprovePost($listingId: Uuid!) {
    approveListing(listingId: $listingId) {
      id
      status
    }
  }
"#;

pub const REJECT_POST: &str = r#"
  mutation RejectPost($listingId: Uuid!, $reason: String!) {
    rejectListing(listingId: $listingId, reason: $reason)
  }
"#;

pub const DELETE_POST: &str = r#"
  mutation DeletePost($listingId: Uuid!) {
    deleteListing(listingId: $listingId)
  }
"#;

// ============================================================================
// WEBSITE MUTATIONS
// ============================================================================

pub const APPROVE_WEBSITE: &str = r#"
  mutation ApproveWebsite($websiteId: String!) {
    approveWebsite(websiteId: $websiteId) {
      id
      status
    }
  }
"#;

pub const REJECT_WEBSITE: &str = r#"
  mutation RejectWebsite($websiteId: String!, $reason: String!) {
    rejectWebsite(websiteId: $websiteId, reason: $reason) {
      id
      status
    }
  }
"#;

pub const CRAWL_WEBSITE: &str = r#"
  mutation CrawlWebsite($websiteId: Uuid!) {
    crawlWebsite(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
"#;

pub const REGENERATE_POSTS: &str = r#"
  mutation RegeneratePosts($websiteId: Uuid!) {
    regeneratePosts(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
"#;

// ============================================================================
// CHAT MUTATIONS
// ============================================================================

pub const CREATE_CHAT: &str = r#"
  mutation CreateChat($language: String, $withAgent: String) {
    createChat(language: $language, withAgent: $withAgent) {
      id
      containerType
      language
      createdAt
      lastActivityAt
    }
  }
"#;

pub const SEND_MESSAGE: &str = r#"
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
"#;

// ============================================================================
// RESOURCE MUTATIONS
// ============================================================================

pub const APPROVE_RESOURCE: &str = r#"
  mutation ApproveResource($resourceId: String!) {
    approveResource(resourceId: $resourceId) {
      id
      status
    }
  }
"#;

pub const REJECT_RESOURCE: &str = r#"
  mutation RejectResource($resourceId: String!, $reason: String!) {
    rejectResource(resourceId: $resourceId, reason: $reason) {
      id
      status
    }
  }
"#;

// ============================================================================
// EXTRACTION MUTATIONS
// ============================================================================

pub const INGEST_SITE: &str = r#"
  mutation IngestSite($siteUrl: String!, $maxPages: Int) {
    ingestSite(siteUrl: $siteUrl, maxPages: $maxPages) {
      siteUrl
      pagesCrawled
      pagesSummarized
      pagesSkipped
    }
  }
"#;

pub const TRIGGER_EXTRACTION: &str = r#"
  mutation TriggerExtraction($input: TriggerExtractionInput!) {
    triggerExtraction(input: $input) {
      success
      query
      site
      extractions {
        content
        status
        grounding
      }
      error
    }
  }
"#;
