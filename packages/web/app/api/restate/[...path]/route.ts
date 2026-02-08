import { NextRequest, NextResponse } from "next/server";

const RESTATE_INGRESS_URL =
  process.env.RESTATE_INGRESS_URL || "http://localhost:8180";
const RESTATE_AUTH_TOKEN = process.env.RESTATE_AUTH_TOKEN || "";

const JWT_SECRET = process.env.JWT_SECRET || "";

// Routes that don't require authentication
const PUBLIC_PATHS = [
  "Auth/send_otp",
  "Auth/verify_otp",
  "Posts/submit",
  "Posts/submit_resource_link",
];

/**
 * Minimal JWT validation (checks signature + expiry)
 * Uses Web Crypto API (Edge Runtime compatible)
 */
async function validateJwt(token: string): Promise<boolean> {
  if (!JWT_SECRET) return false;

  try {
    const parts = token.split(".");
    if (parts.length !== 3) return false;

    const [headerB64, payloadB64, signatureB64] = parts;

    // Verify signature using HMAC-SHA256
    const encoder = new TextEncoder();
    const key = await crypto.subtle.importKey(
      "raw",
      encoder.encode(JWT_SECRET),
      { name: "HMAC", hash: "SHA-256" },
      false,
      ["verify"]
    );

    const data = encoder.encode(`${headerB64}.${payloadB64}`);
    // Convert base64url to standard base64
    const sig = signatureB64.replace(/-/g, "+").replace(/_/g, "/");
    const sigBytes = Uint8Array.from(atob(sig), (c) => c.charCodeAt(0));

    const valid = await crypto.subtle.verify("HMAC", key, sigBytes, data);
    if (!valid) return false;

    // Check expiry
    const payload = JSON.parse(
      atob(payloadB64.replace(/-/g, "+").replace(/_/g, "/"))
    );
    if (payload.exp && payload.exp < Date.now() / 1000) return false;

    return true;
  } catch {
    return false;
  }
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  const restatePath = path.join("/");

  // Check if this is a public route
  const isPublic = PUBLIC_PATHS.some((p) => restatePath.startsWith(p));

  // Get token from cookie or Authorization header
  const cookieToken = request.cookies.get("auth_token")?.value;
  const headerToken = request.headers
    .get("authorization")
    ?.replace("Bearer ", "");
  const token = headerToken || cookieToken;

  // Require auth for non-public routes
  if (!isPublic) {
    if (!token) {
      return NextResponse.json(
        { message: "Authentication required" },
        { status: 401 }
      );
    }

    const valid = await validateJwt(token);
    if (!valid) {
      return NextResponse.json(
        { message: "Invalid or expired token" },
        { status: 401 }
      );
    }
  }

  // Forward to Restate ingress
  const url = `${RESTATE_INGRESS_URL}/${restatePath}`;

  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };

  // Restate Cloud requires its own auth token for ingress
  if (RESTATE_AUTH_TOKEN) {
    headers["Authorization"] = `Bearer ${RESTATE_AUTH_TOKEN}`;
  }

  // Pass user's JWT as a separate header for the backend to read
  if (token) {
    headers["X-User-Token"] = token;
  }

  try {
    const body = await request.text();

    const response = await fetch(url, {
      method: "POST",
      headers,
      body: body || "{}",
    });

    const responseBody = await response.text();

    return new NextResponse(responseBody, {
      status: response.status,
      headers: {
        "Content-Type": response.headers.get("Content-Type") || "application/json",
      },
    });
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Internal server error";
    return NextResponse.json({ message }, { status: 502 });
  }
}
