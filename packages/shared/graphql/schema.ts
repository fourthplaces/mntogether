export const typeDefs = /* GraphQL */ `
type Query {
  publicPosts(
    postType: String
    category: String
    limit: Int
    offset: Int
    zipCode: String
    radiusMiles: Float
  ): PublicPostConnection!
  publicFilters: PublicFilters!
  post(id: ID!): Post
  posts(
    status: String
    search: String
    postType: String
    submissionType: String
    zipCode: String
    radiusMiles: Float
    limit: Int
    offset: Int
  ): PostConnection!

  postStats(status: String): PostStats!

  # Organizations (admin)
  organizations: [Organization!]!
  organization(id: ID!): Organization
  organizationChecklist(id: ID!): Checklist!

  # Sources (admin)
  sources(
    status: String
    sourceType: String
    search: String
    limit: Int
    offset: Int
  ): SourceConnection!
  source(id: ID!): Source
  sourcePages(sourceId: ID!): [ExtractionPage!]!
  sourcePageCount(sourceId: ID!): Int!
  sourceAssessment(sourceId: ID!): Assessment
  searchSourcesByContent(query: String!, limit: Int): SourceConnection!
  extractionPage(url: String!): ExtractionPage
  workflowStatus(workflowName: String!, workflowId: String!): String

  # Websites (admin, legacy)
  websites(
    status: String
    search: String
    limit: Int
    offset: Int
  ): WebsiteConnection!
  website(id: ID!): Website
  websitePages(domain: String!, limit: Int): [ExtractionPage!]!
  websitePageCount(domain: String!): Int!
  websiteAssessment(websiteId: ID!): Assessment
  websitePosts(
    websiteId: ID!
    limit: Int
  ): PostConnection!

  # Tags (admin)
  tagKinds: [TagKind!]!
  tags(kind: String): [Tag!]!

  # Sync/Proposals (admin)
  syncBatches(status: String, limit: Int): SyncBatchConnection!
  syncProposals(batchId: ID!): SyncProposalConnection!

  # Search Queries (admin)
  searchQueries: [SearchQuery!]!

  # Jobs (admin)
  jobs(status: String, limit: Int): [Job!]!

  # Entity Proposals & Notes (admin)
  entityProposals(entityId: ID!): [EntityProposal!]!
  entityNotes(noteableType: String!, noteableId: ID!): [Note!]!

  # Organization detail queries (admin)
  organizationSources(organizationId: ID!): [Source!]!
  organizationPosts(organizationId: ID!, limit: Int): PostConnection!

  # Public organizations
  publicOrganizations: [PublicOrganization!]!
  publicOrganization(id: ID!): PublicOrganization

  # Chat (admin)
  recentChats(limit: Int): [ChatroomInfo!]!
  chatMessages(chatroomId: ID!): [ChatMessage!]!
}

type Mutation {
  trackPostView(postId: ID!): Boolean
  trackPostClick(postId: ID!): Boolean

  # Posts (admin)
  approvePost(id: ID!): Post!
  rejectPost(id: ID!, reason: String): Post!
  archivePost(id: ID!): Post!
  deletePost(id: ID!): Boolean!
  reactivatePost(id: ID!): Post!
  addPostTag(postId: ID!, tagKind: String!, tagValue: String!, displayName: String): Post!
  removePostTag(postId: ID!, tagId: ID!): Post!
  regeneratePost(id: ID!): Post!
  regeneratePostTags(id: ID!): Post!
  updatePostCapacity(id: ID!, capacityStatus: String!): Post!
  batchScorePosts(limit: Int): BatchScoreResult!
  submitResourceLink(url: String!, context: String, submitterContact: String): SubmitResourceResult!
  addComment(postId: ID!, content: String!, parentMessageId: String): Comment!

  # Organizations (admin)
  createOrganization(name: String!, description: String): Organization!
  updateOrganization(id: ID!, name: String!, description: String): Organization!
  deleteOrganization(id: ID!): Boolean!
  approveOrganization(id: ID!): Organization!
  rejectOrganization(id: ID!, reason: String!): Organization!
  suspendOrganization(id: ID!, reason: String!): Organization!
  setOrganizationStatus(id: ID!, status: String!, reason: String): Organization!
  toggleChecklistItem(organizationId: ID!, checklistKey: String!, checked: Boolean!): Checklist!
  regenerateOrganization(id: ID!): RegenerateOrgResult!
  extractOrgPosts(id: ID!): Boolean!
  cleanUpOrgPosts(id: ID!): Boolean!
  runCurator(id: ID!): Boolean!
  removeAllOrgPosts(id: ID!): Boolean!
  removeAllOrgNotes(id: ID!): Boolean!
  rewriteNarratives(organizationId: ID!): RewriteNarrativesResult!

  # Websites (admin, legacy)
  submitNewWebsite(url: String!): Website!
  approveWebsite(id: ID!): Website!
  rejectWebsite(id: ID!, reason: String!): Website!
  crawlWebsite(id: ID!): Boolean!
  generateWebsiteAssessment(id: ID!): Boolean!
  regenerateWebsitePosts(id: ID!): WorkflowStartResult!
  deduplicateWebsitePosts(id: ID!): WorkflowStartResult!
  extractWebsiteOrganization(id: ID!): Website!
  assignWebsiteOrganization(id: ID!, organizationId: ID!): Website!
  unassignWebsiteOrganization(id: ID!): Website!
  approvePostInline(id: ID!): Post!
  rejectPostInline(id: ID!, reason: String): Post!

  # Sources (admin)
  submitWebsite(url: String!): Source!
  lightCrawlAll: LightCrawlAllResult!
  approveSource(id: ID!): Source!
  rejectSource(id: ID!, reason: String!): Source!
  crawlSource(id: ID!): Boolean!
  generateSourceAssessment(id: ID!): Boolean!
  regenerateSourcePosts(id: ID!): WorkflowStartResult!
  deduplicateSourcePosts(id: ID!): WorkflowStartResult!
  extractSourceOrganization(id: ID!): Source!
  assignSourceOrganization(id: ID!, organizationId: ID!): Source!
  unassignSourceOrganization(id: ID!): Source!

  # Tags (admin)
  createTagKind(slug: String!, displayName: String!, description: String, required: Boolean, isPublic: Boolean, allowedResourceTypes: [String!]): TagKind!
  updateTagKind(id: ID!, displayName: String, description: String, required: Boolean, isPublic: Boolean, allowedResourceTypes: [String!]): TagKind!
  deleteTagKind(id: ID!): Boolean!
  createTag(kind: String!, value: String!, displayName: String, color: String, description: String, emoji: String): Tag!
  updateTag(id: ID!, displayName: String, color: String, description: String, emoji: String): Tag!
  deleteTag(id: ID!): Boolean!

  # Sync/Proposals (admin)
  approveProposal(id: ID!): Boolean!
  rejectProposal(id: ID!): Boolean!
  approveBatch(id: ID!): Boolean!
  rejectBatch(id: ID!): Boolean!
  refineProposal(proposalId: ID!, comment: String!): Boolean!

  # Search Queries (admin)
  createSearchQuery(queryText: String!): SearchQuery!
  updateSearchQuery(id: ID!, queryText: String!): SearchQuery!
  toggleSearchQuery(id: ID!): SearchQuery!
  deleteSearchQuery(id: ID!): Boolean!
  runScheduledDiscovery: Boolean!

  # Notes (admin)
  createNote(noteableType: String!, noteableId: ID!, content: String!, severity: String, isPublic: Boolean, ctaText: String, sourceUrl: String): Note!
  updateNote(id: ID!, content: String!, severity: String, isPublic: Boolean, ctaText: String, sourceUrl: String, expiredAt: String): Note!
  deleteNote(id: ID!): Boolean!
  unlinkNote(noteId: ID!, postId: ID!): Boolean!
  generateNotesFromSources(organizationId: ID!): GenerateNotesResult!
  autoAttachNotes(organizationId: ID!): AutoAttachNotesResult!

  # Organization source operations (admin)
  createSocialSource(organizationId: ID!, platform: String!, identifier: String!): Source!
  crawlAllOrgSources(organizationId: ID!): Boolean!

  # Chat (admin)
  createChat(language: String, withAgent: String): ChatroomInfo!
  sendChatMessage(chatroomId: ID!, content: String!): ChatMessage!
}

type PublicFilters {
  categories: [FilterOption!]!
  postTypes: [PostTypeOption!]!
}

type FilterOption {
  value: String!
  displayName: String!
  count: Int!
}

type PostTypeOption {
  value: String!
  displayName: String!
  description: String
  color: String
  emoji: String
}

type Post {
  id: ID!
  title: String!
  description: String!
  descriptionMarkdown: String
  summary: String
  status: String!
  postType: String
  category: String
  capacityStatus: String
  urgency: String
  location: String
  sourceUrl: String
  submissionType: String
  createdAt: String!
  updatedAt: String!
  publishedAt: String
  organizationId: ID
  organizationName: String
  distanceMiles: Float
  relevanceScore: Float
  relevanceBreakdown: String
  hasUrgentNotes: Boolean
  tags: [Tag!]!
  schedules: [PostSchedule!]!
  contacts: [PostContact!]!
  submittedBy: SubmittedByInfo
  urgentNotes: [UrgentNote!]!
  comments: [Comment!]!
}

type PostConnection {
  posts: [Post!]!
  totalCount: Int!
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
}

type PublicPostConnection {
  posts: [PublicPost!]!
  totalCount: Int!
}

type PublicPost {
  id: ID!
  title: String!
  summary: String
  description: String!
  location: String
  sourceUrl: String
  postType: String!
  category: String!
  createdAt: String!
  publishedAt: String
  distanceMiles: Float
  organizationId: ID
  organizationName: String
  tags: [PublicTag!]!
  urgentNotes: [UrgentNote!]!
}

type PublicTag {
  kind: String!
  value: String!
  displayName: String
  color: String
}

type Tag {
  id: ID!
  kind: String!
  value: String!
  displayName: String
  color: String
  description: String
  emoji: String
}

type PostStats {
  total: Int!
  services: Int!
  opportunities: Int!
  businesses: Int!
  userSubmitted: Int!
  scraped: Int!
}

type BatchScoreResult {
  scored: Int!
  failed: Int!
  remaining: Int!
}

type SubmitResourceResult {
  message: String!
  jobId: String
}

type Organization {
  id: ID!
  name: String!
  description: String
  status: String!
  websiteCount: Int!
  socialProfileCount: Int!
  snapshotCount: Int!
  createdAt: String!
  updatedAt: String!
}

type PublicOrganization {
  id: ID!
  name: String!
  description: String
  status: String!
  posts: [PublicPost!]!
}

type Checklist {
  items: [ChecklistItem!]!
  allChecked: Boolean!
}

type ChecklistItem {
  key: String!
  label: String!
  checked: Boolean!
  checkedBy: String
  checkedAt: String
}

type RegenerateOrgResult {
  organizationId: ID
  status: String!
}

type RewriteNarrativesResult {
  rewritten: Int!
  failed: Int!
  total: Int!
}

type Website {
  id: ID!
  domain: String!
  status: String!
  active: Boolean!
  crawlCount: Int
  postCount: Int
  lastCrawledAt: String
  organizationId: ID
  createdAt: String
}

type WebsiteConnection {
  websites: [Website!]!
  totalCount: Int!
  hasNextPage: Boolean!
}

type Source {
  id: ID!
  sourceType: String!
  identifier: String!
  url: String
  status: String!
  active: Boolean!
  organizationId: ID
  organizationName: String
  scrapeFrequencyHours: Int!
  lastScrapedAt: String
  postCount: Int
  snapshotCount: Int
  createdAt: String!
  updatedAt: String!
}

type SourceConnection {
  sources: [Source!]!
  totalCount: Int!
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
}

type ExtractionPage {
  url: String!
  content: String
}

type Assessment {
  id: ID!
  websiteId: ID!
  assessmentMarkdown: String!
  confidenceScore: Float
}

type LightCrawlAllResult {
  sourcesQueued: Int!
}

type WorkflowStartResult {
  workflowId: String!
  status: String!
}

type TagKind {
  id: ID!
  slug: String!
  displayName: String!
  description: String
  allowedResourceTypes: [String!]!
  required: Boolean!
  isPublic: Boolean!
  tagCount: Int!
}

type PostSchedule {
  id: ID!
  dayOfWeek: Int
  opensAt: String
  closesAt: String
  timezone: String!
  notes: String
  rrule: String
  dtstart: String
  dtend: String
  isAllDay: Boolean!
  durationMinutes: Int
}

type PostContact {
  id: ID!
  contactType: String!
  contactValue: String!
  contactLabel: String
}

type SubmittedByInfo {
  submitterType: String!
  agentId: String
  agentName: String
}

type UrgentNote {
  content: String!
  ctaText: String
}

type Comment {
  id: ID!
  containerId: String!
  role: String!
  content: String!
  parentMessageId: String
  createdAt: String!
}

type SyncBatch {
  id: ID!
  resourceType: String!
  sourceId: ID
  sourceName: String
  status: String!
  summary: String
  proposalCount: Int!
  approvedCount: Int!
  rejectedCount: Int!
  createdAt: String!
  reviewedAt: String
}

type SyncBatchConnection {
  batches: [SyncBatch!]!
}

type SyncProposal {
  id: ID!
  batchId: ID!
  operation: String!
  status: String!
  entityType: String!
  draftEntityId: ID
  targetEntityId: ID
  reason: String
  reviewedBy: String
  reviewedAt: String
  createdAt: String!
  draftTitle: String
  targetTitle: String
  mergeSourceIds: [String!]!
  mergeSourceTitles: [String!]!
  relevanceScore: Float
  curatorReasoning: String
  confidence: String
  sourceUrls: [String!]
  revisionCount: Int
}

type SyncProposalConnection {
  proposals: [SyncProposal!]!
}

type EntityProposal {
  id: ID!
  batchId: ID!
  operation: String!
  status: String!
  entityType: String!
  draftEntityId: ID
  targetEntityId: ID
  reason: String
  createdAt: String!
}

type Note {
  id: ID!
  content: String!
  ctaText: String
  severity: String!
  sourceUrl: String
  sourceId: String
  sourceType: String
  isPublic: Boolean!
  createdBy: String!
  expiredAt: String
  createdAt: String!
  updatedAt: String!
  linkedPosts: [LinkedPost!]
}

type LinkedPost {
  id: ID!
  title: String!
}

type GenerateNotesResult {
  notesCreated: Int!
  sourcesScanned: Int!
  postsAttached: Int!
}

type AutoAttachNotesResult {
  notesCount: Int!
  postsCount: Int!
  noteablesCreated: Int!
}

type SearchQuery {
  id: ID!
  queryText: String!
  isActive: Boolean!
  sortOrder: Int!
}

type Job {
  id: ID!
  workflowName: String!
  workflowKey: String!
  status: String!
  progress: String
  createdAt: String
  modifiedAt: String
  completedAt: String
  completionResult: String
  websiteDomain: String
  websiteId: String
}

type ChatroomInfo {
  id: ID!
  title: String
  createdAt: String!
  messageCount: Int!
}

type ChatMessage {
  id: ID!
  chatroomId: String!
  senderType: String!
  content: String!
  createdAt: String!
}
`;
