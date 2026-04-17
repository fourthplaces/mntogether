import { graphql } from "@/gql";
import "./fragments";

export const PostStatsQuery = graphql(`
  query PostStats($status: String) {
    postStats(status: $status) {
      total
      stories
      notices
      exchanges
      events
      spotlights
      references
      userSubmitted
    }
  }
`);

export const PostsListQuery = graphql(`
  query PostsList(
    $status: String
    $search: String
    $postType: String
    $zipCode: String
    $radiusMiles: Float
    $limit: Int
    $offset: Int
  ) {
    posts(
      status: $status
      search: $search
      postType: $postType
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

export const SignalPostsQuery = graphql(`
  query SignalPosts(
    $status: String
    $search: String
    $postType: String
    $countyId: ID
    $statewideOnly: Boolean
    $limit: Int
    $offset: Int
  ) {
    posts(
      status: $status
      search: $search
      postType: $postType
      submissionType: "scraped"
      countyId: $countyId
      statewideOnly: $statewideOnly
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

export const EditionPostsQuery = graphql(`
  query EditionPosts(
    $editionId: ID!
    $slottedFilter: String
    $limit: Int
    $offset: Int
  ) {
    editionPosts(
      editionId: $editionId
      slottedFilter: $slottedFilter
      limit: $limit
      offset: $offset
    ) {
      posts {
        ...PostListFields
      }
      totalCount
    }
  }
`);

export const EditorialPostsQuery = graphql(`
  query EditorialPosts(
    $status: String
    $search: String
    $postType: String
    $limit: Int
    $offset: Int
  ) {
    posts(
      status: $status
      search: $search
      postType: $postType
      excludeSubmissionType: "scraped"
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

export const AddPostContactMutation = graphql(`
  mutation AddPostContact($postId: ID!, $contactType: String!, $contactValue: String!, $contactLabel: String) {
    addPostContact(postId: $postId, contactType: $contactType, contactValue: $contactValue, contactLabel: $contactLabel) {
      id
      contacts { id contactType contactValue contactLabel }
    }
  }
`);

export const RemovePostContactMutation = graphql(`
  mutation RemovePostContact($postId: ID!, $contactId: ID!) {
    removePostContact(postId: $postId, contactId: $contactId) {
      id
      contacts { id contactType contactValue contactLabel }
    }
  }
`);

export const AddPostScheduleMutation = graphql(`
  mutation AddPostSchedule($postId: ID!, $input: AddScheduleInput!) {
    addPostSchedule(postId: $postId, input: $input) {
      id
    }
  }
`);

export const DeletePostScheduleMutation = graphql(`
  mutation DeletePostSchedule($postId: ID!, $scheduleId: ID!) {
    deletePostSchedule(postId: $postId, scheduleId: $scheduleId) {
      id
    }
  }
`);

export const RegeneratePostMutation = graphql(`
  mutation RegeneratePost($id: ID!) {
    regeneratePost(id: $id) {
      id
      title
      bodyRaw
    }
  }
`);

export const CreatePostMutation = graphql(`
  mutation CreatePost($input: CreatePostInput!) {
    createPost(input: $input) {
      id
      title
      status
      postType
    }
  }
`);

export const UpdatePostMutation = graphql(`
  mutation UpdatePost($id: ID!, $input: UpdatePostInput!) {
    updatePost(id: $id, input: $input) {
      ...PostDetailFields
    }
  }
`);

// Field group upsert mutations
export const UpsertPostMediaMutation = graphql(`
  mutation UpsertPostMedia($postId: ID!, $imageUrl: String, $caption: String, $credit: String, $mediaId: ID) {
    upsertPostMedia(postId: $postId, imageUrl: $imageUrl, caption: $caption, credit: $credit, mediaId: $mediaId)
  }
`);

export const UpsertPostMetaMutation = graphql(`
  mutation UpsertPostMeta($postId: ID!, $kicker: String, $byline: String, $deck: String, $updated: String) {
    upsertPostMeta(postId: $postId, kicker: $kicker, byline: $byline, deck: $deck, updated: $updated)
  }
`);

export const UpsertPostPersonMutation = graphql(`
  mutation UpsertPostPerson($postId: ID!, $name: String, $role: String, $bio: String, $photoUrl: String, $quote: String, $photoMediaId: ID) {
    upsertPostPerson(postId: $postId, name: $name, role: $role, bio: $bio, photoUrl: $photoUrl, quote: $quote, photoMediaId: $photoMediaId)
  }
`);

export const UpsertPostLinkMutation = graphql(`
  mutation UpsertPostLink($postId: ID!, $label: String, $url: String, $deadline: String) {
    upsertPostLink(postId: $postId, label: $label, url: $url, deadline: $deadline)
  }
`);

export const UpsertPostSourceAttrMutation = graphql(`
  mutation UpsertPostSourceAttr($postId: ID!, $sourceName: String, $attribution: String) {
    upsertPostSourceAttr(postId: $postId, sourceName: $sourceName, attribution: $attribution)
  }
`);

export const UpsertPostDatetimeMutation = graphql(`
  mutation UpsertPostDatetime($postId: ID!, $startAt: String, $endAt: String, $cost: String, $recurring: Boolean) {
    upsertPostDatetime(postId: $postId, startAt: $startAt, endAt: $endAt, cost: $cost, recurring: $recurring)
  }
`);

export const UpsertPostStatusMutation = graphql(`
  mutation UpsertPostStatus($postId: ID!, $state: String, $verified: String) {
    upsertPostStatus(postId: $postId, state: $state, verified: $verified)
  }
`);

export const UpsertPostItemsMutation = graphql(`
  mutation UpsertPostItems($postId: ID!, $items: [PostItemInput!]!) {
    upsertPostItems(postId: $postId, items: $items)
  }
`);

