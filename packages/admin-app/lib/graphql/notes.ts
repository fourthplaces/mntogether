import { graphql } from "@/gql";
import "./fragments";

export const EntityProposalsQuery = graphql(`
  query EntityProposals($entityId: ID!) {
    entityProposals(entityId: $entityId) {
      id
      batchId
      operation
      status
      entityType
      draftEntityId
      targetEntityId
      reason
      createdAt
    }
  }
`);

export const EntityNotesQuery = graphql(`
  query EntityNotes($noteableType: String!, $noteableId: ID!) {
    entityNotes(noteableType: $noteableType, noteableId: $noteableId) {
      ...NoteFields
    }
  }
`);

export const CreateNoteMutation = graphql(`
  mutation CreateNote(
    $noteableType: String!
    $noteableId: ID!
    $content: String!
    $severity: String
    $isPublic: Boolean
    $ctaText: String
    $sourceUrl: String
  ) {
    createNote(
      noteableType: $noteableType
      noteableId: $noteableId
      content: $content
      severity: $severity
      isPublic: $isPublic
      ctaText: $ctaText
      sourceUrl: $sourceUrl
    ) {
      id
    }
  }
`);

export const UpdateNoteMutation = graphql(`
  mutation UpdateNote(
    $id: ID!
    $content: String!
    $severity: String
    $isPublic: Boolean
    $ctaText: String
    $sourceUrl: String
    $expiredAt: String
  ) {
    updateNote(
      id: $id
      content: $content
      severity: $severity
      isPublic: $isPublic
      ctaText: $ctaText
      sourceUrl: $sourceUrl
      expiredAt: $expiredAt
    ) {
      id
    }
  }
`);

export const DeleteNoteMutation = graphql(`
  mutation DeleteNote($id: ID!) {
    deleteNote(id: $id)
  }
`);

export const UnlinkNoteMutation = graphql(`
  mutation UnlinkNote($noteId: ID!, $postId: ID!) {
    unlinkNote(noteId: $noteId, postId: $postId)
  }
`);

export const GenerateNotesFromSourcesMutation = graphql(`
  mutation GenerateNotesFromSources($organizationId: ID!) {
    generateNotesFromSources(organizationId: $organizationId) {
      notesCreated
      sourcesScanned
      postsAttached
    }
  }
`);

export const AutoAttachNotesMutation = graphql(`
  mutation AutoAttachNotes($organizationId: ID!) {
    autoAttachNotes(organizationId: $organizationId) {
      notesCount
      postsCount
      noteablesCreated
    }
  }
`);

export const CreateSocialSourceMutation = graphql(`
  mutation CreateSocialSource(
    $organizationId: ID!
    $platform: String!
    $identifier: String!
  ) {
    createSocialSource(
      organizationId: $organizationId
      platform: $platform
      identifier: $identifier
    ) {
      id
      sourceType
      identifier
    }
  }
`);

export const CrawlAllOrgSourcesMutation = graphql(`
  mutation CrawlAllOrgSources($organizationId: ID!) {
    crawlAllOrgSources(organizationId: $organizationId)
  }
`);

export const OrganizationSourcesQuery = graphql(`
  query OrganizationSources($organizationId: ID!) {
    organizationSources(organizationId: $organizationId) {
      ...SourceFields
    }
  }
`);

export const OrganizationPostsQuery = graphql(`
  query OrganizationPosts($organizationId: ID!, $limit: Int) {
    organizationPosts(organizationId: $organizationId, limit: $limit) {
      posts {
        id
        title
        status
        postType
        category
        capacityStatus
        createdAt
        organizationId
        organizationName
        relevanceScore
        tags {
          id
          kind
          value
          displayName
          color
        }
      }
      totalCount
      hasNextPage
      hasPreviousPage
    }
  }
`);
