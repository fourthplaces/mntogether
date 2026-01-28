import { gql } from '@apollo/client';

export const GET_PUBLISHED_POSTS = gql`
  query GetPublishedPosts($limit: Int) {
    publishedPosts(limit: $limit) {
      id
      needId
      status
      publishedAt
      expiresAt
      customTitle
      customDescription
      customTldr
      need {
        id
        organizationName
        title
        tldr
        description
        contactInfo {
          email
          phone
          website
        }
        location
        urgency
        createdAt
      }
    }
  }
`;
