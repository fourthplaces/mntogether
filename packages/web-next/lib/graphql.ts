// GraphQL client for server-side data fetching

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/graphql";

export interface GraphQLResponse<T> {
  data?: T;
  errors?: Array<{ message: string }>;
}

export async function graphqlFetch<T>(
  query: string,
  variables?: Record<string, any>
): Promise<T> {
  const response = await fetch(API_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      query,
      variables,
    }),
    next: {
      revalidate: 60, // Revalidate every 60 seconds
    },
  });

  if (!response.ok) {
    throw new Error(`GraphQL request failed: ${response.statusText}`);
  }

  const json: GraphQLResponse<T> = await response.json();

  if (json.errors) {
    throw new Error(json.errors[0].message);
  }

  if (!json.data) {
    throw new Error("No data returned from GraphQL");
  }

  return json.data;
}

// Example queries
export const SEARCH_ORGANIZATIONS = `
  query SearchOrganizationsSemantic($query: String!, $limit: Int) {
    searchOrganizationsSemantic(query: $query, limit: $limit) {
      organization {
        id
        name
        description
        summary
        website
        phone
        primaryAddress
      }
      similarityScore
    }
  }
`;

export const GET_PUBLISHED_POSTS = `
  query GetPublishedPosts($limit: Int) {
    publishedPosts(limit: $limit) {
      id
      title
      description
      organizationName
      createdAt
    }
  }
`;
