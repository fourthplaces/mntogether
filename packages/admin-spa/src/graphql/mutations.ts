import { gql } from '@apollo/client';

export const APPROVE_NEED = gql`
  mutation ApproveNeed($needId: ID!) {
    approveNeed(needId: $needId) {
      id
      status
    }
  }
`;

export const EDIT_AND_APPROVE_NEED = gql`
  mutation EditAndApproveNeed($needId: ID!, $input: EditNeedInput!) {
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
  mutation RejectNeed($needId: ID!, $reason: String!) {
    rejectNeed(needId: $needId, reason: $reason)
  }
`;

export const SUBMIT_NEED = gql`
  mutation SubmitNeed($input: SubmitNeedInput!, $volunteerId: ID) {
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
