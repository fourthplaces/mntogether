import { graphql } from "@/gql";

export const DashboardQuery = graphql(`
  query Dashboard {
    websites(limit: 1000) {
      websites {
        id
        domain
        status
        organizationId
        postCount
        lastCrawledAt
      }
      totalCount
      hasNextPage
    }
    pendingPosts: posts(status: "pending_approval", limit: 1000) {
      posts {
        id
        status
        createdAt
      }
      totalCount
    }
    allPosts: posts(limit: 1000) {
      posts {
        id
        status
        createdAt
      }
      totalCount
    }
  }
`);
