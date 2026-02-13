import { createSchema } from "graphql-yoga";
import { resolvers } from "./resolvers";

const typeDefs = /* GraphQL */ `
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

  # Tags (admin)
  tagKinds: [TagKind!]!
  tags(kind: String): [Tag!]!
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

  # Tags (admin)
  createTagKind(slug: String!, displayName: String!, description: String, required: Boolean, isPublic: Boolean, allowedResourceTypes: [String!]): TagKind!
  updateTagKind(id: ID!, displayName: String, description: String, required: Boolean, isPublic: Boolean, allowedResourceTypes: [String!]): TagKind!
  deleteTagKind(id: ID!): Boolean!
  createTag(kind: String!, value: String!, displayName: String, color: String, description: String, emoji: String): Tag!
  updateTag(id: ID!, displayName: String, color: String, description: String, emoji: String): Tag!
  deleteTag(id: ID!): Boolean!
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
`;

export const schema = createSchema({ typeDefs, resolvers });
