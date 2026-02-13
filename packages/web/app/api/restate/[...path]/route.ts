import { NextRequest, NextResponse } from "next/server";

const RESTATE_INGRESS_URL =
  process.env.RESTATE_INGRESS_URL || "http://localhost:8180";
const RESTATE_AUTH_TOKEN = process.env.RESTATE_AUTH_TOKEN || "";

// Internal-only paths that cannot be called through the proxy.
// These are scheduled/cron handlers invoked by Restate's send_after(),
// which bypasses the proxy entirely (direct to port 9080).
const INTERNAL_ONLY_PATHS = [
  "Posts/expire_stale_posts",
  "Websites/run_scheduled_scrape",
  "Websites/run_scheduled_discovery",
  "Sources/run_scheduled_scrape",
  "Sources/run_scheduled_discovery",
  "Organizations/run_scheduled_extraction",
  "Members/run_weekly_reset",
  "HeatMap/compute_snapshot",
  "SocialProfiles/run_scheduled_scrape",
];

// Routes that don't require authentication
const PUBLIC_PATHS = [
  "Auth/send_otp",
  "Auth/verify_otp",
  "Posts/submit",
  "Posts/submit_resource_link",
  "Posts/public_list",
  "Posts/public_filters",
  "Organizations/public_list",
  "Organizations/public_get",
];

// Virtual object paths that are publicly readable (Post/{uuid}/get)
const PUBLIC_OBJECT_PATTERNS = [
  /^Post\/[^/]+\/get$/,
  /^Post\/[^/]+\/track_view$/,
  /^Post\/[^/]+\/track_click$/,
  /^Post\/[^/]+\/get_comments$/,
];

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  const restatePath = path.join("/");
  // Block internal-only paths (scheduled/cron handlers) from external access
  const isInternal = INTERNAL_ONLY_PATHS.some((p) => restatePath.startsWith(p));
  if (isInternal) {
    console.log(`[restate-proxy] 403 internal-only path: ${restatePath}`);
    return NextResponse.json(
      { message: "This endpoint is not externally accessible" },
      { status: 403 }
    );
  }

  const isPublic = PUBLIC_PATHS.some((p) => restatePath.startsWith(p)) ||
    PUBLIC_OBJECT_PATTERNS.some((r) => r.test(restatePath));

  // Token comes from httpOnly cookie (set during login, sent automatically by browser)
  const token = request.cookies.get("auth_token")?.value;

  console.log(`[restate-proxy] ${restatePath} | public=${isPublic} | auth=${token ? "yes" : "no"}`);

  // Require a token for non-public routes (actual JWT validation happens in the backend)
  if (!isPublic && !token) {
    console.log(`[restate-proxy] 401 no token for ${restatePath}`);
    return NextResponse.json(
      { message: "Authentication required" },
      { status: 401 }
    );
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

    if (!response.ok) {
      console.log(`[restate-proxy] ${restatePath} -> ${response.status}: ${responseBody.slice(0, 200)}`);
    }

    return new NextResponse(responseBody, {
      status: response.status,
      headers: {
        "Content-Type": response.headers.get("Content-Type") || "application/json",
      },
    });
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Internal server error";
    console.error(`[restate-proxy] ${restatePath} fetch error:`, message);
    return NextResponse.json({ message }, { status: 502 });
  }
}
