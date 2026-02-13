export const baseTypeDefs = /* GraphQL */ `
type Query {
  # Public
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

  # Admin (auth required)
  posts(
    status: String
    search: String
    limit: Int
    offset: Int
  ): PostConnection!
}

type Mutation {
  # Public
  trackPostView(postId: ID!): Boolean
  trackPostClick(postId: ID!): Boolean
}

# Shared types
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
`;

export const postTypeDefs = /* GraphQL */ `
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

  # Inline nested data (returned by Restate)
  tags: [Tag!]!
  schedules: [PostSchedule!]!
  contacts: [PostContact!]!
  submittedBy: SubmittedByInfo
  urgentNotes: [UrgentNote!]!

  # Dataloader-resolved relations
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
