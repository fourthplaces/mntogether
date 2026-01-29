import { gql } from '@apollo/client';

// Public mutations
export const SUBMIT_LISTING = gql`
  mutation SubmitListing($input: SubmitListingInput!) {
    submitListing(input: $input) {
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

// Admin mutations
export const APPROVE_LISTING = gql`
  mutation ApproveListing($listingId: Uuid!) {
    approveListing(listingId: $listingId) {
      id
      status
    }
  }
`;

export const EDIT_AND_APPROVE_LISTING = gql`
  mutation EditAndApproveListing($listingId: Uuid!, $input: EditListingInput!) {
    editAndApproveListing(listingId: $listingId, input: $input) {
      id
      title
      description
      tldr
      status
    }
  }
`;

export const REJECT_LISTING = gql`
  mutation RejectListing($listingId: Uuid!, $reason: String!) {
    rejectListing(listingId: $listingId, reason: $reason)
  }
`;

export const SEND_VERIFICATION_CODE = gql`
  mutation SendVerificationCode($phoneNumber: String!) {
    sendVerificationCode(phoneNumber: $phoneNumber)
  }
`;

export const VERIFY_CODE = gql`
  mutation VerifyCode($phoneNumber: String!, $code: String!) {
    verifyCode(phoneNumber: $phoneNumber, code: $code)
  }
`;

export const SCRAPE_ORGANIZATION = gql`
  mutation ScrapeOrganization($sourceId: Uuid!) {
    scrapeOrganization(sourceId: $sourceId) {
      jobId
      status
      message
    }
  }
`;

export const EXPIRE_POST = gql`
  mutation ExpirePost($postId: Uuid!) {
    expirePost(postId: $postId) {
      id
      status
      expiresAt
    }
  }
`;

export const ARCHIVE_POST = gql`
  mutation ArchivePost($postId: Uuid!) {
    archivePost(postId: $postId) {
      id
      status
    }
  }
`;

export const DELETE_LISTING = gql`
  mutation DeleteListing($listingId: Uuid!) {
    deleteListing(listingId: $listingId)
  }
`;

export const ADD_ORGANIZATION_SCRAPE_URL = gql`
  mutation AddOrganizationScrapeUrl($sourceId: Uuid!, $url: String!) {
    addOrganizationScrapeUrl(sourceId: $sourceId, url: $url)
  }
`;

export const REMOVE_ORGANIZATION_SCRAPE_URL = gql`
  mutation RemoveOrganizationScrapeUrl($sourceId: Uuid!, $url: String!) {
    removeOrganizationScrapeUrl(sourceId: $sourceId, url: $url)
  }
`;
