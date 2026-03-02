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
  }
`);
