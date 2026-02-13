import { graphql } from "@/gql";

export const JobsListQuery = graphql(`
  query JobsList($status: String, $limit: Int) {
    jobs(status: $status, limit: $limit) {
      id
      workflowName
      workflowKey
      status
      progress
      createdAt
      modifiedAt
      completedAt
      completionResult
      websiteDomain
      websiteId
    }
  }
`);
