import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

const AUTH_COOKIE_NAME = "auth_token";

// Routes that require authentication
const protectedRoutes = [
  "/admin/dashboard",
  "/admin/posts",
  "/admin/websites",
  "/admin/extraction",
  "/admin/resources",
  "/admin/organizations",
];

// Routes that should redirect authenticated users (e.g., login page)
const authRoutes = ["/admin/login"];

/**
 * Decode JWT payload and check if expired.
 * No signature verification — just reads the exp claim.
 * Real auth validation happens in the Restate backend.
 */
function isTokenExpired(token: string): boolean {
  try {
    const payload = token.split(".")[1];
    if (!payload) return true;
    const decoded = JSON.parse(atob(payload.replace(/-/g, "+").replace(/_/g, "/")));
    return !decoded.exp || decoded.exp < Date.now() / 1000;
  } catch {
    return true;
  }
}

export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl;
  const token = request.cookies.get(AUTH_COOKIE_NAME)?.value;
  const isAuthenticated = token && !isTokenExpired(token);

  // Redirect /admin to /admin/dashboard
  if (pathname === "/admin" || pathname === "/admin/") {
    return NextResponse.redirect(new URL("/admin/dashboard", request.url));
  }

  // Check if the current path starts with any protected route
  const isProtectedRoute = protectedRoutes.some(
    (route) => pathname === route || pathname.startsWith(`${route}/`)
  );

  // Check if the current path is an auth route (login)
  const isAuthRoute = authRoutes.some(
    (route) => pathname === route || pathname.startsWith(`${route}/`)
  );

  // Redirect unauthenticated or expired users from protected routes to login
  if (isProtectedRoute && !isAuthenticated) {
    const response = NextResponse.redirect(
      new URL(`/admin/login?redirect=${pathname}`, request.url)
    );
    // Clear stale cookie if token was expired
    if (token) {
      response.cookies.delete(AUTH_COOKIE_NAME);
    }
    return response;
  }

  // Redirect authenticated users from auth routes to dashboard
  if (isAuthRoute && isAuthenticated) {
    const redirectUrl = request.nextUrl.searchParams.get("redirect");
    const destination = redirectUrl || "/admin/dashboard";
    return NextResponse.redirect(new URL(destination, request.url));
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    // Match all admin routes
    "/admin/:path*",
  ],
};
