// Server-side API client for Next.js Server Actions
// Calls the Rust Axum server directly

import { cookies } from "next/headers";

const API_URL = process.env.API_URL || "http://localhost:9080";

/**
 * Get the auth token from cookies (server-side)
 */
export async function getAuthToken(): Promise<string | null> {
  const cookieStore = await cookies();
  return cookieStore.get("auth_token")?.value ?? null;
}

/**
 * Call a service handler from the server
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

  // Pass user's JWT for the backend to read
  if (token) {
    headers["X-User-Token"] = token;
  }

  const url = `${API_URL}/${path}`;
  console.log(`[api] API_URL=${process.env.API_URL || "(NOT SET)"}`);
  console.log(`[api] POST ${url}`);

  let response: Response;
  try {
    response = await fetch(url, {
      method: "POST",
      headers,
      body: body !== undefined ? JSON.stringify(body) : "{}",
      cache: "no-store",
    });
  } catch (err) {
    console.error(`[api] fetch failed for ${url}:`, err);
    throw err;
  }

  if (!response.ok) {
    const text = await response.text();
    let message: string;
    try {
      const json = JSON.parse(text);
      message = json.message || json.error || text;
    } catch {
      message = text;
    }
    throw new Error(message || `API request failed: ${response.statusText}`);
  }

  return response.json();
}

/**
 * Call a keyed object handler from the server
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
