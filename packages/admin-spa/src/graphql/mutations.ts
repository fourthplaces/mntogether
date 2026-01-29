import { gql } from '@apollo/client';

export const APPROVE_NEED = gql`
  mutation ApproveNeed($needId: Uuid!) {
    approveNeed(needId: $needId) {
      id
      status
    }
  }
`;

export const EDIT_AND_APPROVE_NEED = gql`
  mutation EditAndApproveNeed($needId: Uuid!, $input: EditNeedInput!) {
    editAndApproveNeed(needId: $needId, input: $input) {
      id
      title
      description
      tldr
      status
    }
  }
`;

export const REJECT_NEED = gql`
  mutation RejectNeed($needId: Uuid!, $reason: String!) {
    rejectNeed(needId: $needId, reason: $reason)
  }
`;

export const SUBMIT_NEED = gql`
  mutation SubmitNeed($input: SubmitNeedInput!, $volunteerId: Uuid) {
    submitNeed(input: $input, volunteerId: $volunteerId) {
      id
      status
    }
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

export const SUBMIT_RESOURCE_LINK = gql`
  mutation SubmitResourceLink($input: SubmitResourceLinkInput!) {
    submitResourceLink(input: $input) {
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

export const DELETE_NEED = gql`
  mutation DeleteNeed($needId: Uuid!) {
    deleteNeed(needId: $needId)
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
