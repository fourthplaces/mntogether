"use client";

// Client-side Restate client with SWR support

import useSWR, { SWRConfiguration, mutate as globalMutate } from "swr";

/**
 * Build the proxy URL for a Restate service call
 */
function serviceUrl(service: string, handler: string): string {
  return `/api/restate/${service}/${handler}`;
}

/**
 * Build the proxy URL for a Restate virtual object call
 */
function objectUrl(object: string, key: string, handler: string): string {
  return `/api/restate/${object}/${key}/${handler}`;
}

/**
 * Low-level fetch to the Restate proxy
 */
async function restateFetch<T>(path: string, body?: unknown): Promise<T> {
  // Auth cookie is httpOnly — sent automatically by the browser.
  // The proxy reads it server-side and forwards to Restate.
  const response = await fetch(path, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: body !== undefined ? JSON.stringify(body) : "{}",
  });

  if (!response.ok) {
    const text = await response.text();
    let message: string;
    try {
      const json = JSON.parse(text);
      message = json.message || json.error || text;
    } catch {
      message = text;
    }
    throw new Error(message || `Request failed: ${response.statusText}`);
  }

  const text = await response.text();
  if (!text) return undefined as T;
  return JSON.parse(text);
}

// --- SWR hooks ---

type SwrKey = { path: string; body: unknown };

function createSwrKey(path: string, body?: unknown): string {
  return JSON.stringify({ path, body });
}

async function swrFetcher<T>(key: string): Promise<T> {
  const { path, body }: SwrKey = JSON.parse(key);
  return restateFetch<T>(path, body);
}

/**
 * SWR hook for Restate service calls (queries)
 *
 * Usage:
 *   const { data } = useRestate<PostList>("Posts", "list", { status: "pending_approval" });
 */
export function useRestate<T>(
  service: string | null,
  handler: string,
  body?: unknown,
  config?: SWRConfiguration<T>
) {
  const path = service ? serviceUrl(service, handler) : null;
  const swrKey = path ? createSwrKey(path, body) : null;

  return useSWR<T>(swrKey, swrFetcher, {
    revalidateOnFocus: false,
    ...config,
  });
}

/**
 * SWR hook for Restate virtual object calls (queries)
 *
 * Usage:
 *   const { data } = useRestateObject<Post>("Post", postId, "get", {});
 */
export function useRestateObject<T>(
  object: string,
  key: string | null,
  handler: string,
  body?: unknown,
  config?: SWRConfiguration<T>
) {
  const path = key ? objectUrl(object, key, handler) : null;
  const swrKey = path ? createSwrKey(path, body) : null;

  return useSWR<T>(swrKey, swrFetcher, {
    revalidateOnFocus: false,
    ...config,
  });
}

// --- Mutation helpers ---

/**
 * Call a Restate service handler (mutations)
 *
 * Usage:
 *   await callService("Posts", "submit", { url: "..." });
 */
export async function callService<T>(
  service: string,
  handler: string,
  body?: unknown
): Promise<T> {
  return restateFetch<T>(serviceUrl(service, handler), body);
}

/**
 * Call a Restate virtual object handler (mutations)
 *
 * Usage:
 *   await callObject("Post", postId, "approve", {});
 */
export async function callObject<T>(
  object: string,
  key: string,
  handler: string,
  body?: unknown
): Promise<T> {
  return restateFetch<T>(objectUrl(object, key, handler), body);
}

// --- Cache invalidation ---

/**
 * Invalidate all SWR cache entries for a given service
 */
export function invalidateService(service: string) {
  globalMutate(
    (key) => {
      if (typeof key !== "string") return false;
      try {
        const parsed: SwrKey = JSON.parse(key);
        return parsed.path.startsWith(`/api/restate/${service}/`);
      } catch {
        return false;
      }
    },
    undefined,
    { revalidate: true }
  );
}

/**
 * Invalidate all SWR cache entries for a given virtual object
 */
export function invalidateObject(object: string, key?: string) {
  const prefix = key
    ? `/api/restate/${object}/${key}/`
    : `/api/restate/${object}/`;
  globalMutate(
    (swrKey) => {
      if (typeof swrKey !== "string") return false;
      try {
        const parsed: SwrKey = JSON.parse(swrKey);
        return parsed.path.startsWith(prefix);
      } catch {
        return false;
      }
    },
    undefined,
    { revalidate: true }
  );
}
