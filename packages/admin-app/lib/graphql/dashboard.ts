import { graphql } from "@/gql";

export const DashboardQuery = graphql(`
  query Dashboard {
    latestEditions {
      id
      county {
        id
        name
      }
      periodStart
      periodEnd
      status
      publishedAt
      rows {
        id
      }
      createdAt
    }
    pendingPosts: posts(status: "pending_approval", limit: 5) {
      posts {
        id
        title
        status
        createdAt
      }
      totalCount
    }
    allPosts: posts(limit: 1) {
      totalCount
    }
  }
`);
