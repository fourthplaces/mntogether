import { NextRequest } from "next/server";

const SSE_BASE =
  process.env.SSE_SERVER_URL || "http://localhost:8081";

/**
 * SSE proxy — reads the httpOnly auth_token cookie and forwards
 * it as a query parameter to the Rust SSE server.
 *
 * This keeps the JWT secure (not accessible from client JS) while
 * still authenticating SSE connections.
 */
export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ topic: string }> }
) {
  const token = request.cookies.get("auth_token")?.value;
  if (!token) {
    return new Response("Unauthorized", { status: 401 });
  }

  const { topic } = await params;
  const sseUrl = `${SSE_BASE}/api/streams/${encodeURIComponent(topic)}?token=${encodeURIComponent(token)}`;

  const upstream = await fetch(sseUrl, {
    headers: { Accept: "text/event-stream" },
  });

  if (!upstream.ok) {
    return new Response(upstream.statusText, { status: upstream.status });
  }

  return new Response(upstream.body, {
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
    },
  });
}
