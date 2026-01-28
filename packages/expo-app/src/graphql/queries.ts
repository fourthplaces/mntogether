import { gql } from '@apollo/client';

export const GET_ACTIVE_NEEDS = gql`
  query GetActiveNeeds($limit: Int, $offset: Int) {
    needs(status: ACTIVE, limit: $limit, offset: $offset) {
      nodes {
        id
        organizationName
        title
        tldr
        location
        urgency
        createdAt
      }
      totalCount
      hasNextPage
    }
  }
`;

export const GET_NEED_DETAIL = gql`
  query GetNeedDetail($id: ID!) {
    need(id: $id) {
      id
      organizationName
      title
      tldr
      description
      descriptionMarkdown
      contactInfo {
        email
        phone
        website
      }
      urgency
      location
      createdAt
    }
  }
`;
