// Server-side Restate client for Next.js Server Actions
// Calls Restate ingress directly (no proxy needed server-side)
// Replaces lib/graphql/server.ts

import { cookies } from "next/headers";

const RESTATE_INGRESS_URL =
  process.env.RESTATE_INGRESS_URL || "http://localhost:8180";

/**
 * Get the auth token from cookies (server-side)
 */
export async function getAuthToken(): Promise<string | null> {
  const cookieStore = await cookies();
  return cookieStore.get("auth_token")?.value ?? null;
}

/**
 * Call a Restate service handler from the server
 *
 * Usage:
 *   const result = await restateCall<OtpSent>("Auth/send_otp", { phone_number: "..." });
 */
export async function restateCall<T>(
  path: string,
  body?: unknown
): Promise<T> {
  const token = await getAuthToken();

  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const url = `${RESTATE_INGRESS_URL}/${path}`;

  const response = await fetch(url, {
    method: "POST",
    headers,
    body: body !== undefined ? JSON.stringify(body) : "{}",
    cache: "no-store",
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
    throw new Error(message || `Restate request failed: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Call a Restate virtual object handler from the server
 *
 * Usage:
 *   const result = await restateObjectCall<Post>("Post", postId, "get", {});
 */
export async function restateObjectCall<T>(
  object: string,
  key: string,
  handler: string,
  body?: unknown
): Promise<T> {
  return restateCall<T>(`${object}/${key}/${handler}`, body);
}
