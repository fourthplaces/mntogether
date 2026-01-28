import { gql } from '@apollo/client';

export const SUBMIT_NEED = gql`
  mutation SubmitNeed($input: SubmitNeedInput!, $volunteerId: ID) {
    submitNeed(input: $input, volunteerId: $volunteerId) {
      id
      status
    }
  }
`;

export const REGISTER_VOLUNTEER = gql`
  mutation RegisterVolunteer($input: RegisterVolunteerInput!) {
    registerVolunteer(input: $input) {
      id
      expoPushToken
      searchableText
    }
  }
`;
