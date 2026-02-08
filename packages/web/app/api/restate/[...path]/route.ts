import { NextRequest, NextResponse } from "next/server";

const RESTATE_INGRESS_URL =
  process.env.RESTATE_INGRESS_URL || "http://localhost:8180";
const RESTATE_AUTH_TOKEN = process.env.RESTATE_AUTH_TOKEN || "";

// Routes that don't require authentication
const PUBLIC_PATHS = [
  "Auth/send_otp",
  "Auth/verify_otp",
  "Posts/submit",
  "Posts/submit_resource_link",
];

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> }
) {
  const { path } = await params;
  const restatePath = path.join("/");
  const isPublic = PUBLIC_PATHS.some((p) => restatePath.startsWith(p));

  // Token comes from httpOnly cookie (set during login, sent automatically by browser)
  const token = request.cookies.get("auth_token")?.value;

  console.log(`[restate-proxy] ${restatePath} | public=${isPublic} | token=${token ? `${token.slice(0, 20)}...` : "none"}`);

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
