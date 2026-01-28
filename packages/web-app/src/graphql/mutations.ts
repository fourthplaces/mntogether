import { gql } from '@apollo/client';

export const SUBMIT_NEED = gql`
  mutation SubmitNeed($input: SubmitNeedInput!) {
    submitNeed(input: $input) {
      id
      status
    }
  }
`;

export const TRACK_POST_VIEW = gql`
  mutation TrackPostView($postId: ID!) {
    postViewed(postId: $postId)
  }
`;

export const TRACK_POST_CLICK = gql`
  mutation TrackPostClick($postId: ID!) {
    postClicked(postId: $postId)
  }
`;

export const SUBMIT_RESOURCE_LINK = gql`
  mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
    submitResourceLink(input: $input) {
      jobId
      status
      message
    }
  }
`;
