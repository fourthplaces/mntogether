import { graphql } from "@/gql";
import "./fragments";

export const SignalInboxQuery = graphql(`
  query SignalInbox($limit: Int, $offset: Int) {
    signalInbox(limit: $limit, offset: $offset) {
      totalCount
      rows {
        reviewFlags
        post {
          id
          title
          bodyRaw
          bodyLight
          status
          postType
          weight
          isUrgent
          location
          createdAt
          publishedAt
          submissionType
          organizationId
          organizationName
          duplicateOfId
          sourceUrl
          tags {
            id
            kind
            value
            displayName
            color
          }
          meta {
            kicker
            byline
            deck
          }
        }
      }
    }
  }
`);

export const SignalInboxBadgeQuery = graphql(`
  query SignalInboxBadge {
    signalInbox(limit: 1) {
      totalCount
    }
  }
`);

export const SignalInboxCanonicalQuery = graphql(`
  query SignalInboxCanonical($id: ID!) {
    post(id: $id) {
      id
      title
      bodyRaw
      bodyLight
      postType
      weight
      location
      publishedAt
      organizationName
      sourceUrl
      status
    }
  }
`);
