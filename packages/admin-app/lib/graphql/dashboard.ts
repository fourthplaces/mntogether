import { graphql } from "@/gql";

export const DashboardQuery = graphql(`
  query Dashboard {
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
