import { graphql } from "@/gql";
import "./fragments";

export const NotesListQuery = graphql(`
  query NotesList($severity: String, $isPublic: Boolean, $limit: Int, $offset: Int) {
    notes(severity: $severity, isPublic: $isPublic, limit: $limit, offset: $offset) {
      notes {
        ...NoteFields
      }
      totalCount
    }
  }
`);

export const NoteDetailQuery = graphql(`
  query NoteDetail($id: ID!) {
    note(id: $id) {
      ...NoteFields
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

export const AutoAttachNotesMutation = graphql(`
  mutation AutoAttachNotes($organizationId: ID!) {
    autoAttachNotes(organizationId: $organizationId) {
      notesCount
      postsCount
      noteablesCreated
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
