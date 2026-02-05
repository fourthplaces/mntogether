// GraphQL Mutations for web-next
// These are plain strings (no gql tag needed) for use with fetch-based client

// ============================================================================
// PUBLIC MUTATIONS
// ============================================================================

export const SUBMIT_POST = `
  mutation SubmitPost($input: SubmitListingInput!) {
    submitListing(input: $input) {
      id
      status
    }
  }
`;

export const TRACK_POST_VIEW = `
  mutation TrackPostView($postId: ID!) {
    postViewed(postId: $postId)
  }
`;

export const TRACK_POST_CLICK = `
  mutation TrackPostClick($postId: ID!) {
    postClicked(postId: $postId)
  }
`;

export const SUBMIT_RESOURCE_LINK = `
  mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
    submitResourceLink(input: $input) {
      jobId
      status
      message
    }
  }
`;

// ============================================================================
// AUTHENTICATION MUTATIONS
// ============================================================================

export const SEND_VERIFICATION_CODE = `
  mutation SendVerificationCode($phoneNumber: String!) {
    sendVerificationCode(phoneNumber: $phoneNumber)
  }
`;

export const VERIFY_CODE = `
  mutation VerifyCode($phoneNumber: String!, $code: String!) {
    verifyCode(phoneNumber: $phoneNumber, code: $code)
  }
`;

export const LOGOUT = `
  mutation Logout($sessionToken: String!) {
    logout(sessionToken: $sessionToken)
  }
`;

// ============================================================================
// ADMIN POST MUTATIONS
// ============================================================================

export const APPROVE_POST = `
  mutation ApprovePost($listingId: Uuid!) {
    approveListing(listingId: $listingId) {
      id
      status
    }
  }
`;

export const EDIT_AND_APPROVE_POST = `
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

export const REJECT_POST = `
  mutation RejectPost($listingId: Uuid!, $reason: String!) {
    rejectListing(listingId: $listingId, reason: $reason)
  }
`;

export const EXPIRE_POST = `
  mutation ExpirePost($postId: Uuid!) {
    expirePost(postId: $postId) {
      id
      status
      expiresAt
    }
  }
`;

export const ARCHIVE_POST = `
  mutation ArchivePost($postId: Uuid!) {
    archivePost(postId: $postId) {
      id
      status
    }
  }
`;

export const DELETE_POST = `
  mutation DeletePost($listingId: Uuid!) {
    deleteListing(listingId: $listingId)
  }
`;

// ============================================================================
// POST TAG MUTATIONS
// ============================================================================

export const UPDATE_POST_TAGS = `
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

export const ADD_POST_TAG = `
  mutation AddPostTag($listingId: Uuid!, $tagKind: String!, $tagValue: String!, $displayName: String) {
    addListingTag(listingId: $listingId, tagKind: $tagKind, tagValue: $tagValue, displayName: $displayName) {
      id
      kind
      value
      displayName
    }
  }
`;

export const REMOVE_POST_TAG = `
  mutation RemovePostTag($listingId: Uuid!, $tagId: String!) {
    removeListingTag(listingId: $listingId, tagId: $tagId)
  }
`;

// ============================================================================
// WEBSITE MUTATIONS
// ============================================================================

export const APPROVE_WEBSITE = `
  mutation ApproveWebsite($websiteId: String!) {
    approveWebsite(websiteId: $websiteId) {
      id
      status
    }
  }
`;

export const REJECT_WEBSITE = `
  mutation RejectWebsite($websiteId: String!, $reason: String!) {
    rejectWebsite(websiteId: $websiteId, reason: $reason) {
      id
      status
    }
  }
`;

export const SUSPEND_WEBSITE = `
  mutation SuspendWebsite($websiteId: String!, $reason: String!) {
    suspendWebsite(websiteId: $websiteId, reason: $reason) {
      id
      status
    }
  }
`;

export const GENERATE_WEBSITE_ASSESSMENT = `
  mutation GenerateWebsiteAssessment($websiteId: String!) {
    generateWebsiteAssessment(websiteId: $websiteId)
  }
`;

export const CRAWL_WEBSITE = `
  mutation CrawlWebsite($websiteId: Uuid!) {
    crawlWebsite(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

export const DISCOVER_WEBSITE = `
  mutation DiscoverWebsite($websiteId: Uuid!) {
    discoverWebsite(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

export const REGENERATE_POSTS = `
  mutation RegeneratePosts($websiteId: Uuid!) {
    regeneratePosts(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

export const REGENERATE_POST = `
  mutation RegeneratePost($postId: Uuid!) {
    regeneratePost(postId: $postId) {
      jobId
      status
      message
    }
  }
`;

export const REGENERATE_PAGE_SUMMARIES = `
  mutation RegeneratePageSummaries($websiteId: Uuid!) {
    regeneratePageSummaries(websiteId: $websiteId) {
      jobId
      status
      message
    }
  }
`;

export const SCRAPE_ORGANIZATION = `
  mutation ScrapeOrganization($sourceId: Uuid!) {
    scrapeOrganization(sourceId: $sourceId) {
      jobId
      status
      message
    }
  }
`;

export const ADD_ORGANIZATION_SCRAPE_URL = `
  mutation AddOrganizationScrapeUrl($sourceId: Uuid!, $url: String!) {
    addOrganizationScrapeUrl(sourceId: $sourceId, url: $url)
  }
`;

export const REMOVE_ORGANIZATION_SCRAPE_URL = `
  mutation RemoveOrganizationScrapeUrl($sourceId: Uuid!, $url: String!) {
    removeOrganizationScrapeUrl(sourceId: $sourceId, url: $url)
  }
`;

// ============================================================================
// CHAT MUTATIONS
// ============================================================================

export const CREATE_CHAT = `
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

export const SEND_MESSAGE = `
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

export const SIGNAL_TYPING = `
  mutation SignalTyping($containerId: String!) {
    signalTyping(containerId: $containerId)
  }
`;

// ============================================================================
// RESOURCE MUTATIONS
// ============================================================================

export const APPROVE_RESOURCE = `
  mutation ApproveResource($resourceId: String!) {
    approveResource(resourceId: $resourceId) {
      id
      status
    }
  }
`;

export const REJECT_RESOURCE = `
  mutation RejectResource($resourceId: String!, $reason: String!) {
    rejectResource(resourceId: $resourceId, reason: $reason) {
      id
      status
    }
  }
`;

export const EDIT_RESOURCE = `
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

export const EDIT_AND_APPROVE_RESOURCE = `
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

export const DELETE_RESOURCE = `
  mutation DeleteResource($resourceId: String!) {
    deleteResource(resourceId: $resourceId)
  }
`;

export const GENERATE_MISSING_EMBEDDINGS = `
  mutation GenerateMissingEmbeddings($batchSize: Int) {
    generateMissingEmbeddings(batchSize: $batchSize) {
      processed
      failed
      remaining
    }
  }
`;

export const GENERATE_POST_EMBEDDING = `
  mutation GeneratePostEmbedding($postId: Uuid!) {
    generatePostEmbedding(postId: $postId)
  }
`;

// ============================================================================
// EXTRACTION MUTATIONS
// ============================================================================

export const SUBMIT_URL = `
  mutation SubmitUrl($input: SubmitUrlInput!) {
    submitUrl(input: $input) {
      success
      url
      extraction {
        content
        status
        grounding
        sources {
          url
          title
          fetchedAt
          role
        }
        gaps {
          field
          suggestedQuery
          isSearchable
        }
        conflicts {
          topic
          claims {
            statement
            sourceUrl
          }
        }
      }
      error
    }
  }
`;

export const TRIGGER_EXTRACTION = `
  mutation TriggerExtraction($input: TriggerExtractionInput!) {
    triggerExtraction(input: $input) {
      success
      query
      site
      extractions {
        content
        status
        grounding
        sources {
          url
          title
          fetchedAt
          role
        }
        gaps {
          field
          suggestedQuery
          isSearchable
        }
        conflicts {
          topic
          claims {
            statement
            sourceUrl
          }
        }
      }
      error
    }
  }
`;

export const INGEST_SITE = `
  mutation IngestSite($siteUrl: String!, $maxPages: Int) {
    ingestSite(siteUrl: $siteUrl, maxPages: $maxPages) {
      siteUrl
      pagesCrawled
      pagesSummarized
      pagesSkipped
    }
  }
`;

// ============================================================================
// DISCOVERY MUTATIONS
// ============================================================================

export const RUN_DISCOVERY_SEARCH = `
  mutation RunDiscoverySearch {
    runDiscoverySearch {
      queriesRun
      totalResults
      websitesCreated
      websitesFiltered
      runId
    }
  }
`;

export const CREATE_DISCOVERY_QUERY = `
  mutation CreateDiscoveryQuery($queryText: String!, $category: String) {
    createDiscoveryQuery(queryText: $queryText, category: $category) {
      id
      queryText
      category
      isActive
      createdAt
    }
  }
`;

export const UPDATE_DISCOVERY_QUERY = `
  mutation UpdateDiscoveryQuery($id: Uuid!, $queryText: String!, $category: String) {
    updateDiscoveryQuery(id: $id, queryText: $queryText, category: $category) {
      id
      queryText
      category
      isActive
    }
  }
`;

export const TOGGLE_DISCOVERY_QUERY = `
  mutation ToggleDiscoveryQuery($id: Uuid!, $isActive: Boolean!) {
    toggleDiscoveryQuery(id: $id, isActive: $isActive) {
      id
      isActive
    }
  }
`;

export const DELETE_DISCOVERY_QUERY = `
  mutation DeleteDiscoveryQuery($id: Uuid!) {
    deleteDiscoveryQuery(id: $id)
  }
`;

export const CREATE_DISCOVERY_FILTER_RULE = `
  mutation CreateDiscoveryFilterRule($queryId: Uuid, $ruleText: String!) {
    createDiscoveryFilterRule(queryId: $queryId, ruleText: $ruleText) {
      id
      queryId
      ruleText
      sortOrder
      isActive
    }
  }
`;

export const UPDATE_DISCOVERY_FILTER_RULE = `
  mutation UpdateDiscoveryFilterRule($id: Uuid!, $ruleText: String!) {
    updateDiscoveryFilterRule(id: $id, ruleText: $ruleText) {
      id
      ruleText
    }
  }
`;

export const DELETE_DISCOVERY_FILTER_RULE = `
  mutation DeleteDiscoveryFilterRule($id: Uuid!) {
    deleteDiscoveryFilterRule(id: $id)
  }
`;
