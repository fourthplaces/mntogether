// Server-side GraphQL client for Next.js Server Components and Server Actions
import { cookies } from "next/headers";

// Server-side: Use API_URL (internal Docker network) if available, otherwise NEXT_PUBLIC_API_URL
const API_URL = process.env.API_URL || process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/graphql";

export interface GraphQLResponse<T> {
  data?: T;
  errors?: Array<{ message: string; path?: string[] }>;
}

export interface GraphQLFetchOptions {
  revalidate?: number | false;
  cache?: RequestCache;
  tags?: string[];
}

/**
 * Get the auth token from HTTP-only cookies (server-side)
 */
export async function getAuthToken(): Promise<string | null> {
  const cookieStore = await cookies();
  return cookieStore.get("auth_token")?.value ?? null;
}

/**
 * Server-side GraphQL fetch with Next.js caching support
 * Automatically includes auth token from cookies
 */
export async function graphqlFetch<T>(
  query: string,
  variables?: Record<string, unknown>,
  options?: GraphQLFetchOptions
): Promise<T> {
  const token = await getAuthToken();

  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(API_URL, {
    method: "POST",
    headers,
    body: JSON.stringify({
      query,
      variables,
    }),
    next: {
      revalidate: options?.revalidate ?? 60,
      tags: options?.tags,
    },
    cache: options?.cache,
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
 * Server-side GraphQL fetch that requires authentication
 * Throws if no auth token is present
 */
export async function graphqlFetchAuth<T>(
  query: string,
  variables?: Record<string, unknown>,
  options?: GraphQLFetchOptions
): Promise<T> {
  const token = await getAuthToken();

  if (!token) {
    throw new Error("Authentication required");
  }

  return graphqlFetch<T>(query, variables, options);
}

/**
 * Server-side GraphQL mutation (no caching)
 */
export async function graphqlMutate<T>(
  mutation: string,
  variables?: Record<string, unknown>
): Promise<T> {
  return graphqlFetch<T>(mutation, variables, { cache: "no-store" });
}
