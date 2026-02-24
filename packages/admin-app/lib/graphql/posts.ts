import { graphql } from "@/gql";
import "./fragments";

export const PostStatsQuery = graphql(`
  query PostStats($status: String) {
    postStats(status: $status) {
      total
      services
      opportunities
      businesses
      userSubmitted
      scraped
    }
  }
`);

export const PostsListQuery = graphql(`
  query PostsList(
    $status: String
    $search: String
    $postType: String
    $submissionType: String
    $zipCode: String
    $radiusMiles: Float
    $limit: Int
    $offset: Int
  ) {
    posts(
      status: $status
      search: $search
      postType: $postType
      submissionType: $submissionType
      zipCode: $zipCode
      radiusMiles: $radiusMiles
      limit: $limit
      offset: $offset
    ) {
      posts {
        ...PostListFields
      }
      totalCount
      hasNextPage
      hasPreviousPage
    }
  }
`);

export const PostDetailQuery = graphql(`
  query PostDetail($id: ID!) {
    post(id: $id) {
      ...PostDetailFields
    }
  }
`);

export const PostDetailFullQuery = graphql(`
  query PostDetailFull($id: ID!) {
    post(id: $id) {
      ...PostDetailFields
      organization {
        id
        name
      }
    }
    entityProposals(entityId: $id) {
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
    entityNotes(noteableType: "post", noteableId: $id) {
      ...NoteFields
    }
  }
`);

export const ApprovePostMutation = graphql(`
  mutation ApprovePost($id: ID!) {
    approvePost(id: $id) {
      id
      status
    }
  }
`);

export const RejectPostMutation = graphql(`
  mutation RejectPost($id: ID!, $reason: String) {
    rejectPost(id: $id, reason: $reason) {
      id
      status
    }
  }
`);

export const ArchivePostMutation = graphql(`
  mutation ArchivePost($id: ID!) {
    archivePost(id: $id) {
      id
      status
    }
  }
`);

export const DeletePostMutation = graphql(`
  mutation DeletePost($id: ID!) {
    deletePost(id: $id)
  }
`);

export const ReactivatePostMutation = graphql(`
  mutation ReactivatePost($id: ID!) {
    reactivatePost(id: $id) {
      id
      status
    }
  }
`);

export const AddPostTagMutation = graphql(`
  mutation AddPostTag($postId: ID!, $tagKind: String!, $tagValue: String!, $displayName: String) {
    addPostTag(postId: $postId, tagKind: $tagKind, tagValue: $tagValue, displayName: $displayName) {
      id
      tags {
        id
        kind
        value
        displayName
        color
      }
    }
  }
`);

export const RemovePostTagMutation = graphql(`
  mutation RemovePostTag($postId: ID!, $tagId: ID!) {
    removePostTag(postId: $postId, tagId: $tagId) {
      id
      tags {
        id
        kind
        value
        displayName
        color
      }
    }
  }
`);

export const RegeneratePostMutation = graphql(`
  mutation RegeneratePost($id: ID!) {
    regeneratePost(id: $id) {
      id
      title
      description
      descriptionMarkdown
      summary
    }
  }
`);

export const RegeneratePostTagsMutation = graphql(`
  mutation RegeneratePostTags($id: ID!) {
    regeneratePostTags(id: $id) {
      id
      tags {
        id
        kind
        value
        displayName
        color
      }
    }
  }
`);

export const BatchScorePostsMutation = graphql(`
  mutation BatchScorePosts($limit: Int) {
    batchScorePosts(limit: $limit) {
      scored
      failed
      remaining
    }
  }
`);

export const UpdatePostCapacityMutation = graphql(`
  mutation UpdatePostCapacity($id: ID!, $capacityStatus: String!) {
    updatePostCapacity(id: $id, capacityStatus: $capacityStatus) {
      id
      capacityStatus
    }
  }
`);

export const SubmitResourceLinkMutation = graphql(`
  mutation SubmitResourceLink($url: String!, $context: String, $submitterContact: String) {
    submitResourceLink(url: $url, context: $context, submitterContact: $submitterContact) {
      message
      jobId
    }
  }
`);

export const AddCommentMutation = graphql(`
  mutation AddComment($postId: ID!, $content: String!, $parentMessageId: String) {
    addComment(postId: $postId, content: $content, parentMessageId: $parentMessageId) {
      id
      content
      parentMessageId
      createdAt
    }
  }
`);
