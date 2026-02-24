import { graphql } from "@/gql";

export const TagKindsQuery = graphql(`
  query TagKinds {
    tagKinds {
      id
      slug
      displayName
      description
      allowedResourceTypes
      required
      isPublic
      tagCount
    }
  }
`);

export const TagsQuery = graphql(`
  query Tags {
    tags {
      id
      kind
      value
      displayName
      color
      description
      emoji
    }
  }
`);

export const CreateTagKindMutation = graphql(`
  mutation CreateTagKind(
    $slug: String!
    $displayName: String!
    $description: String
    $required: Boolean
    $isPublic: Boolean
    $allowedResourceTypes: [String!]
  ) {
    createTagKind(
      slug: $slug
      displayName: $displayName
      description: $description
      required: $required
      isPublic: $isPublic
      allowedResourceTypes: $allowedResourceTypes
    ) {
      id
      slug
      displayName
    }
  }
`);

export const UpdateTagKindMutation = graphql(`
  mutation UpdateTagKind(
    $id: ID!
    $displayName: String
    $description: String
    $required: Boolean
    $isPublic: Boolean
    $allowedResourceTypes: [String!]
  ) {
    updateTagKind(
      id: $id
      displayName: $displayName
      description: $description
      required: $required
      isPublic: $isPublic
      allowedResourceTypes: $allowedResourceTypes
    ) {
      id
      slug
      displayName
    }
  }
`);

export const DeleteTagKindMutation = graphql(`
  mutation DeleteTagKind($id: ID!) {
    deleteTagKind(id: $id)
  }
`);

export const CreateTagMutation = graphql(`
  mutation CreateTag(
    $kind: String!
    $value: String!
    $displayName: String
    $color: String
    $description: String
    $emoji: String
  ) {
    createTag(
      kind: $kind
      value: $value
      displayName: $displayName
      color: $color
      description: $description
      emoji: $emoji
    ) {
      id
      kind
      value
      displayName
    }
  }
`);

export const UpdateTagMutation = graphql(`
  mutation UpdateTag(
    $id: ID!
    $displayName: String
    $color: String
    $description: String
    $emoji: String
  ) {
    updateTag(
      id: $id
      displayName: $displayName
      color: $color
      description: $description
      emoji: $emoji
    ) {
      id
      kind
      value
      displayName
    }
  }
`);

export const DeleteTagMutation = graphql(`
  mutation DeleteTag($id: ID!) {
    deleteTag(id: $id)
  }
`);
