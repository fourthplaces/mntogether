import { graphql } from "@/gql";

export const SearchQueriesListQuery = graphql(`
  query SearchQueriesList {
    searchQueries {
      id
      queryText
      isActive
      sortOrder
    }
  }
`);

export const CreateSearchQueryMutation = graphql(`
  mutation CreateSearchQuery($queryText: String!) {
    createSearchQuery(queryText: $queryText) {
      id
      queryText
      isActive
      sortOrder
    }
  }
`);

export const UpdateSearchQueryMutation = graphql(`
  mutation UpdateSearchQuery($id: ID!, $queryText: String!) {
    updateSearchQuery(id: $id, queryText: $queryText) {
      id
      queryText
      isActive
      sortOrder
    }
  }
`);

export const ToggleSearchQueryMutation = graphql(`
  mutation ToggleSearchQuery($id: ID!) {
    toggleSearchQuery(id: $id) {
      id
      queryText
      isActive
      sortOrder
    }
  }
`);

export const DeleteSearchQueryMutation = graphql(`
  mutation DeleteSearchQuery($id: ID!) {
    deleteSearchQuery(id: $id)
  }
`);

export const RunScheduledDiscoveryMutation = graphql(`
  mutation RunScheduledDiscovery {
    runScheduledDiscovery
  }
`);
