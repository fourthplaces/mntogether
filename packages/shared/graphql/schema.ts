export const typeDefs = /* GraphQL */ `
enum PostStatus { pending_approval active rejected archived }
enum PostType { story notice exchange event spotlight reference }
enum Weight { heavy medium light }
enum OrganizationStatus { pending_review approved rejected suspended }

type Query {
  # Posts (public)
  publicPosts(
    postType: String
    category: String
    limit: Int
    offset: Int
    zipCode: String
    radiusMiles: Float
  ): PublicPostConnection!
  publicFilters: PublicFilters!

  # Posts (admin)
  post(id: ID!): Post
  posts(
    status: String
    search: String
    postType: String
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

  # Tags (admin)
  tagKinds: [TagKind!]!
  tags(kind: String): [Tag!]!

  # Jobs (admin)
  jobs(status: String, limit: Int): [Job!]!

  # Notes (admin)
  entityNotes(noteableType: String!, noteableId: ID!): [Note!]!

  # Organization detail queries (admin)
  organizationPosts(organizationId: ID!, limit: Int): PostConnection!

  # Public organizations
  publicOrganizations: [PublicOrganization!]!
  publicOrganization(id: ID!): PublicOrganization

  # Counties + Editions (admin)
  counties: [County!]!
  county(id: ID!): County
  editions(countyId: ID, status: String, limit: Int, offset: Int): EditionConnection!
  edition(id: ID!): Edition
  currentEdition(countyId: ID!): Edition
  rowTemplates: [RowTemplate!]!
  postTemplates: [PostTemplateConfig!]!
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

  # Tags (admin)
  createTagKind(slug: String!, displayName: String!, description: String, required: Boolean, isPublic: Boolean, allowedResourceTypes: [String!]): TagKind!
  updateTagKind(id: ID!, displayName: String, description: String, required: Boolean, isPublic: Boolean, allowedResourceTypes: [String!]): TagKind!
  deleteTagKind(id: ID!): Boolean!
  createTag(kind: String!, value: String!, displayName: String, color: String, description: String, emoji: String): Tag!
  updateTag(id: ID!, displayName: String, color: String, description: String, emoji: String): Tag!
  deleteTag(id: ID!): Boolean!

  # Notes (admin)
  createNote(noteableType: String!, noteableId: ID!, content: String!, severity: String, isPublic: Boolean, ctaText: String, sourceUrl: String): Note!
  updateNote(id: ID!, content: String!, severity: String, isPublic: Boolean, ctaText: String, sourceUrl: String, expiredAt: String): Note!
  deleteNote(id: ID!): Boolean!
  unlinkNote(noteId: ID!, postId: ID!): Boolean!
  autoAttachNotes(organizationId: ID!): AutoAttachNotesResult!

  # Editions (admin)
  createEdition(countyId: ID!, periodStart: String!, periodEnd: String!, title: String): Edition!
  generateEdition(id: ID!): Edition!
  publishEdition(id: ID!): Edition!
  archiveEdition(id: ID!): Edition!
  batchGenerateEditions(periodStart: String!, periodEnd: String!): BatchGenerateEditionsResult!
  updateEditionRow(rowId: ID!, rowTemplateSlug: String, sortOrder: Int): EditionRow!
  reorderEditionRows(editionId: ID!, rowIds: [ID!]!): [EditionRow!]!
  removePostFromEdition(slotId: ID!): Boolean!
  changeSlotTemplate(slotId: ID!, postTemplate: String!): EditionSlot!
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
  status: PostStatus!
  postType: PostType
  weight: Weight
  priority: Int
  category: String
  capacityStatus: String
  urgency: String
  location: String
  sourceUrl: String
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
  organization: Organization
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
  postType: PostType!
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
  stories: Int!
  notices: Int!
  exchanges: Int!
  events: Int!
  spotlights: Int!
  references: Int!
  userSubmitted: Int!
}

type BatchScoreResult {
  scored: Int!
  failed: Int!
  remaining: Int!
}

type Organization {
  id: ID!
  name: String!
  description: String
  status: OrganizationStatus!
  createdAt: String!
  updatedAt: String!
  posts(limit: Int): PostConnection!
  notes: [Note!]!
  checklist: Checklist!
}

type PublicOrganization {
  id: ID!
  name: String!
  description: String
  status: OrganizationStatus!
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

type Note {
  id: ID!
  content: String!
  ctaText: String
  severity: String!
  sourceUrl: String
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

type AutoAttachNotesResult {
  notesCount: Int!
  postsCount: Int!
  noteablesCreated: Int!
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
}

type County {
  id: ID!
  fipsCode: String!
  name: String!
  state: String!
}

type Edition {
  id: ID!
  county: County!
  title: String
  periodStart: String!
  periodEnd: String!
  status: String!
  publishedAt: String
  rows: [EditionRow!]!
  createdAt: String!
}

type EditionConnection {
  editions: [Edition!]!
  totalCount: Int!
}

type EditionRow {
  id: ID!
  rowTemplate: RowTemplate!
  sortOrder: Int!
  slots: [EditionSlot!]!
}

type EditionSlot {
  id: ID!
  post: Post!
  postTemplate: String!
  slotIndex: Int!
}

type RowTemplate {
  id: ID!
  slug: String!
  displayName: String!
  description: String
  slots: [RowTemplateSlotDef!]!
}

type RowTemplateSlotDef {
  slotIndex: Int!
  weight: Weight!
  count: Int!
  accepts: [PostType!]
}

type PostTemplateConfig {
  id: ID!
  slug: String!
  displayName: String!
  description: String
  compatibleTypes: [PostType!]!
  bodyTarget: Int!
  bodyMax: Int!
  titleMax: Int!
}

type BatchGenerateEditionsResult {
  created: Int!
  failed: Int!
  totalCounties: Int!
}
`;
