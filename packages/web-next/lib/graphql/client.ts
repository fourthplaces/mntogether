"use client";

// Client-side GraphQL client with SWR support for React Client Components

import useSWR, { SWRConfiguration, mutate as globalMutate } from "swr";

const API_URL = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/graphql";

export interface GraphQLResponse<T> {
  data?: T;
  errors?: Array<{ message: string; path?: string[] }>;
}

/**
 * Get the auth token from cookies (client-side)
 */
function getAuthTokenClient(): string | null {
  if (typeof document === "undefined") return null;
  const match = document.cookie.match(/(?:^|; )auth_token=([^;]*)/);
  return match ? decodeURIComponent(match[1]) : null;
}

/**
 * Client-side GraphQL fetcher
 */
export async function graphqlFetchClient<T>(
  query: string,
  variables?: Record<string, unknown>,
  token?: string
): Promise<T> {
  const authToken = token ?? getAuthTokenClient();

  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };

  if (authToken) {
    headers["Authorization"] = `Bearer ${authToken}`;
  }

  const response = await fetch(API_URL, {
    method: "POST",
    headers,
    body: JSON.stringify({
      query,
      variables,
    }),
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

/**
 * Create an SWR key from query and variables
 */
function createSwrKey(query: string, variables?: Record<string, unknown>): string {
  return JSON.stringify({ query, variables });
}

/**
 * SWR fetcher for GraphQL queries
 */
async function swrFetcher<T>(key: string): Promise<T> {
  const { query, variables } = JSON.parse(key);
  return graphqlFetchClient<T>(query, variables);
}

/**
 * SWR hook for GraphQL queries
 */
export function useGraphQL<T>(
  query: string,
  variables?: Record<string, unknown>,
  config?: SWRConfiguration<T>
) {
  const key = createSwrKey(query, variables);

  return useSWR<T>(key, swrFetcher, {
    revalidateOnFocus: false,
    ...config,
  });
}

/**
 * GraphQL mutation function for client-side
 * Returns the mutation function and an optional function to revalidate related queries
 */
export async function graphqlMutateClient<T>(
  mutation: string,
  variables?: Record<string, unknown>
): Promise<T> {
  return graphqlFetchClient<T>(mutation, variables);
}

/**
 * Invalidate SWR cache for a specific query
 */
export function invalidateQuery(query: string, variables?: Record<string, unknown>) {
  const key = createSwrKey(query, variables);
  globalMutate(key);
}

/**
 * Invalidate all SWR cache entries that match a query (ignoring variables)
 */
export function invalidateAllMatchingQuery(query: string) {
  globalMutate(
    (key) => {
      if (typeof key !== "string") return false;
      try {
        const parsed = JSON.parse(key);
        return parsed.query === query;
      } catch {
        return false;
      }
    },
    undefined,
    { revalidate: true }
  );
}
