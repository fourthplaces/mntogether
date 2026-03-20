export const typeDefs = /* GraphQL */ `
enum PostStatus { draft active rejected expired archived filled }
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
    submissionType: String
    excludeSubmissionType: String
    countyId: ID
    statewideOnly: Boolean
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

  # Notes (admin)
  note(id: ID!): Note
  notes(severity: String, isPublic: Boolean, limit: Int, offset: Int): NoteConnection!
  entityNotes(noteableType: String!, noteableId: ID!): [Note!]!

  # Organization detail queries (admin)
  organizationPosts(organizationId: ID!, limit: Int): PostConnection!

  # Public organizations
  publicOrganizations: [PublicOrganization!]!
  publicOrganization(id: ID!): PublicOrganization

  # Public broadsheet (no auth)
  publicBroadsheet(countyId: ID!): PublicBroadsheet

  # Broadsheet preview (admin auth required, any edition status)
  editionPreview(editionId: ID!): PublicBroadsheet

  # Counties + Editions (admin)
  countyDashboard: [CountyDashboardRow!]!
  counties: [County!]!
  county(id: ID!): County
  editions(countyId: ID, status: String, periodStart: String, periodEnd: String, limit: Int, offset: Int): EditionConnection!
  latestEditions: [Edition!]!
  edition(id: ID!): Edition
  currentEdition(countyId: ID!): Edition
  editionKanbanStats(periodStart: String!, periodEnd: String!): EditionKanbanStats!
  rowTemplates: [RowTemplate!]!
  postTemplates: [PostTemplateConfig!]!

  # Widgets (admin)
  widget(id: ID!): Widget
  widgets(widgetType: String, countyId: ID, search: String, limit: Int, offset: Int): [Widget!]!
  editionWidgets(editionId: ID!, slottedFilter: String, limit: Int, offset: Int): [Widget!]!

  # Media Library (admin)
  mediaLibrary(limit: Int, offset: Int, contentType: String): MediaConnection!
  presignedUpload(filename: String!, contentType: String!, sizeBytes: Int!): PresignedUpload!
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
  addPostContact(postId: ID!, contactType: String!, contactValue: String!, contactLabel: String): Post!
  removePostContact(postId: ID!, contactId: ID!): Post!
  addPostSchedule(postId: ID!, input: AddScheduleInput!): Post!
  deletePostSchedule(postId: ID!, scheduleId: ID!): Post!
  regeneratePost(id: ID!): Post!
  createPost(input: CreatePostInput!): Post!
  updatePost(id: ID!, input: UpdatePostInput!): Post!

  # Post field group upserts (admin)
  upsertPostMedia(postId: ID!, imageUrl: String, caption: String, credit: String): Boolean!
  upsertPostMeta(postId: ID!, kicker: String, byline: String, deck: String, updated: String): Boolean!
  upsertPostPerson(postId: ID!, name: String, role: String, bio: String, photoUrl: String, quote: String): Boolean!
  upsertPostLink(postId: ID!, label: String, url: String, deadline: String): Boolean!
  upsertPostSourceAttr(postId: ID!, sourceName: String, attribution: String): Boolean!
  upsertPostDatetime(postId: ID!, startAt: String, endAt: String, cost: String, recurring: Boolean): Boolean!
  upsertPostStatus(postId: ID!, state: String, verified: String): Boolean!

  # Organizations (admin)
  createOrganization(name: String!, description: String, sourceType: String): Organization!
  updateOrganization(id: ID!, name: String!, description: String): Organization!
  deleteOrganization(id: ID!): Boolean!
  approveOrganization(id: ID!): Organization!
  rejectOrganization(id: ID!, reason: String!): Organization!
  suspendOrganization(id: ID!, reason: String!): Organization!
  setOrganizationStatus(id: ID!, status: String!, reason: String): Organization!
  toggleChecklistItem(organizationId: ID!, checklistKey: String!, checked: Boolean!): Checklist!
  regenerateOrganization(id: ID!): RegenerateOrgResult!
  addOrgTag(organizationId: ID!, tagKind: String!, tagValue: String!, displayName: String): Organization!
  removeOrgTag(organizationId: ID!, tagId: ID!): Organization!

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
  linkNote(noteId: ID!, noteableType: String!, noteableId: ID!): Note!
  unlinkNote(noteId: ID!, noteableType: String!, noteableId: ID!): Boolean!
  autoAttachNotes(organizationId: ID!): AutoAttachNotesResult!

  # Editions (admin)
  createEdition(countyId: ID!, periodStart: String!, periodEnd: String!, title: String): Edition!
  generateEdition(id: ID!): Edition!
  reviewEdition(id: ID!): Edition!
  approveEdition(id: ID!): Edition!
  publishEdition(id: ID!): Edition!
  archiveEdition(id: ID!): Edition!
  batchGenerateEditions(periodStart: String!, periodEnd: String!): BatchGenerateEditionsResult!
  batchApproveEditions(ids: [ID!]!): BatchEditionsResult!
  batchPublishEditions(ids: [ID!]!): BatchEditionsResult!
  updateEditionRow(rowId: ID!, rowTemplateSlug: String, sortOrder: Int): EditionRow!
  reorderEditionRows(editionId: ID!, rowIds: [ID!]!): [EditionRow!]!
  moveSlot(slotId: ID!, targetRowId: ID!, slotIndex: Int!): EditionSlot!
  addPostToEdition(editionRowId: ID!, postId: ID!, postTemplate: String!, slotIndex: Int!): EditionSlot!
  addEditionRow(editionId: ID!, rowTemplateSlug: String!, sortOrder: Int!): EditionRow!
  deleteEditionRow(rowId: ID!): Boolean!
  removePostFromEdition(slotId: ID!): Boolean!
  changeSlotTemplate(slotId: ID!, postTemplate: String!): EditionSlot!

  # Widgets (admin)
  createWidget(widgetType: String!, data: String!, authoringMode: String, zipCode: String, city: String, countyId: ID, startDate: String, endDate: String): Widget!
  updateWidget(id: ID!, data: String, zipCode: String, city: String, countyId: ID, startDate: String, endDate: String): Widget!
  updateWidgetData(id: ID!, data: String!): Widget!
  deleteWidget(id: ID!): Boolean!
  addWidgetToEdition(editionRowId: ID!, widgetId: ID!, slotIndex: Int!): EditionSlot!

  # Sections (admin)
  addSection(editionId: ID!, title: String!, subtitle: String, topicSlug: String, sortOrder: Int!): EditionSection!
  updateSection(id: ID!, title: String, subtitle: String, topicSlug: String): EditionSection!
  reorderSections(editionId: ID!, sectionIds: [ID!]!): [EditionSection!]!
  deleteSection(id: ID!): Boolean!
  assignRowToSection(rowId: ID!, sectionId: ID): Boolean!

  # Media Library (admin)
  confirmUpload(storageKey: String!, publicUrl: String!, filename: String!, contentType: String!, sizeBytes: Int!, altText: String, width: Int, height: Int): Media!
  deleteMedia(id: ID!): Boolean!
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
  bodyRaw: String!
  bodyAst: String
  status: PostStatus!
  postType: PostType
  weight: Weight
  priority: Int
  category: String
  urgency: String
  location: String
  sourceUrl: String
  createdAt: String!
  updatedAt: String!
  publishedAt: String
  organizationId: ID
  organizationName: String
  distanceMiles: Float
  hasUrgentNotes: Boolean
  tags: [Tag!]!
  schedules: [PostSchedule!]!
  contacts: [PostContact!]!
  submissionType: String
  submittedBy: SubmittedByInfo
  urgentNotes: [UrgentNote!]!
  organization: Organization
  bodyHeavy: String
  bodyMedium: String
  bodyLight: String
  zipCode: String
  latitude: Float
  longitude: Float
  revisionOfPostId: ID
  translationOfId: ID
  duplicateOfId: ID
  sourceLanguage: String!
  # Field groups
  media: [BroadsheetMedia!]!
  items: [BroadsheetItem!]!
  person: BroadsheetPerson
  link: BroadsheetLink
  sourceAttribution: BroadsheetSourceAttribution
  meta: BroadsheetMeta
  datetime: BroadsheetDatetime
  postStatus: BroadsheetStatus
  schedule: [BroadsheetScheduleEntry!]!
  relatedPosts: [PublicPost!]!
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
  bodyRaw: String!
  bodyLight: String
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

type Organization {
  id: ID!
  name: String!
  description: String
  sourceType: String!
  status: OrganizationStatus!
  createdAt: String!
  updatedAt: String!
  tags: [Tag!]!
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
  locked: Boolean!
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
}

type UrgentNote {
  content: String!
  ctaText: String
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
  linkedOrgs: [LinkedOrg!]
}

type LinkedPost {
  id: ID!
  title: String!
}

type LinkedOrg {
  id: ID!
  name: String!
}

type NoteConnection {
  notes: [Note!]!
  totalCount: Int!
}

type AutoAttachNotesResult {
  notesCount: Int!
  postsCount: Int!
  noteablesCreated: Int!
}

type County {
  id: ID!
  fipsCode: String!
  name: String!
  state: String!
}

type CountyDashboardRow {
  county: County!
  currentEdition: Edition
  lastPublishedAt: String
  isStale: Boolean!
}

type Edition {
  id: ID!
  county: County!
  title: String
  periodStart: String!
  periodEnd: String!
  status: String!
  publishedAt: String
  rowCount: Int!
  rows: [EditionRow!]!
  sections: [EditionSection!]!
  createdAt: String!
}

type Widget {
  id: ID!
  widgetType: String!
  authoringMode: String!
  data: String!
  zipCode: String
  city: String
  countyId: ID
  county: County
  startDate: String
  endDate: String
  createdAt: String!
  updatedAt: String!
}

type EditionSection {
  id: ID!
  editionId: ID!
  title: String!
  subtitle: String
  topicSlug: String
  sortOrder: Int!
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
  sectionId: ID
  slots: [EditionSlot!]!
}

type EditionSlot {
  id: ID!
  kind: String!
  slotIndex: Int!
  post: Post
  postTemplate: String
  widget: Widget
}

type RowTemplate {
  id: ID!
  slug: String!
  displayName: String!
  description: String
  layoutVariant: String!
  slots: [RowTemplateSlotDef!]!
}

type RowTemplateSlotDef {
  slotIndex: Int!
  weight: Weight!
  count: Int!
  accepts: [PostType!]
  postTemplateSlug: String
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
  weight: Weight!
}

type BatchGenerateEditionsResult {
  created: Int!
  regenerated: Int!
  skipped: Int!
  failed: Int!
  totalCounties: Int!
}

type BatchEditionsResult {
  succeeded: Int!
  failed: Int!
}

type EditionKanbanStats {
  draft: Int!
  inReview: Int!
  approved: Int!
  published: Int!
}

# ========================================
# Public Broadsheet (rendered homepage)
# ========================================

type PublicBroadsheet {
  id: ID!
  title: String
  periodStart: String!
  periodEnd: String!
  status: String!
  publishedAt: String
  county: BroadsheetCounty!
  rows: [BroadsheetRow!]!
  sections: [BroadsheetSection!]!
}

type BroadsheetSection {
  id: ID!
  title: String!
  subtitle: String
  topicSlug: String
  sortOrder: Int!
}

type BroadsheetCounty {
  id: ID!
  fipsCode: String!
  name: String!
  state: String!
}

type BroadsheetRow {
  rowTemplateSlug: String!
  layoutVariant: String!
  sortOrder: Int!
  sectionId: ID
  slots: [BroadsheetSlot!]!
}

type BroadsheetSlot {
  kind: String!
  slotIndex: Int!
  postTemplate: String
  widgetTemplate: String
  post: BroadsheetPost
  widget: BroadsheetWidget
}

type BroadsheetWidget {
  id: ID!
  widgetType: String!
  authoringMode: String!
  data: String!
}

type BroadsheetPost {
  id: ID!
  title: String!
  bodyRaw: String!
  postType: String!
  weight: String!
  urgency: String
  location: String
  sourceUrl: String
  organizationName: String
  publishedAt: String
  tags: [PublicTag!]!
  contacts: [BroadsheetContact!]!
  urgentNotes: [UrgentNote!]!
  bodyHeavy: String
  bodyMedium: String
  bodyLight: String
  media: [BroadsheetMedia!]!
  items: [BroadsheetItem!]!
  person: BroadsheetPerson
  link: BroadsheetLink
  sourceAttribution: BroadsheetSourceAttribution
  meta: BroadsheetMeta
  datetime: BroadsheetDatetime
  postStatus: BroadsheetStatus
  schedule: [BroadsheetScheduleEntry!]!
}

type BroadsheetContact {
  contactType: String!
  contactValue: String!
  contactLabel: String
}

type BroadsheetMedia {
  imageUrl: String
  caption: String
  credit: String
}

type BroadsheetItem {
  name: String!
  detail: String
}

type BroadsheetPerson {
  name: String
  role: String
  bio: String
  photoUrl: String
  quote: String
}

type BroadsheetLink {
  label: String
  url: String
  deadline: String
}

type BroadsheetSourceAttribution {
  sourceName: String
  attribution: String
}

type BroadsheetMeta {
  kicker: String
  byline: String
  timestamp: String
  updated: String
  deck: String
}

type BroadsheetDatetime {
  start: String
  end: String
  cost: String
  recurring: Boolean
}

type BroadsheetStatus {
  state: String
  verified: String
}

type BroadsheetScheduleEntry {
  day: String!
  opens: String!
  closes: String!
}

input CreatePostInput {
  title: String!
  bodyRaw: String!
  postType: String
  weight: String
  priority: Int
  urgency: String
  location: String
  organizationId: ID
}

input UpdatePostInput {
  title: String
  bodyRaw: String
  bodyAst: String
  postType: String
  category: String
  weight: String
  priority: Int
  urgency: String
  location: String
  zipCode: String
  sourceUrl: String
  organizationId: ID
}

input AddScheduleInput {
  dayOfWeek: Int
  opensAt: String
  closesAt: String
  timezone: String
  notes: String
  rrule: String
  dtstart: String
  dtend: String
  isAllDay: Boolean
  durationMinutes: Int
}

# ========================================
# Media Library
# ========================================

type Media {
  id: ID!
  filename: String!
  contentType: String!
  sizeBytes: Int!
  url: String!
  storageKey: String!
  altText: String
  width: Int
  height: Int
  createdAt: String!
}

type MediaConnection {
  media: [Media!]!
  totalCount: Int!
  hasNextPage: Boolean!
}

type PresignedUpload {
  uploadUrl: String!
  storageKey: String!
  publicUrl: String!
}
`;
