import { graphql } from "@/gql";

export const DashboardQuery = graphql(`
  query Dashboard($periodStart: String!, $periodEnd: String!) {
    editionKanbanStats(periodStart: $periodStart, periodEnd: $periodEnd) {
      draft
      inReview
      approved
      published
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
