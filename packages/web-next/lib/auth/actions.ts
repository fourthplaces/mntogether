"use server";

import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { graphqlFetch } from "../graphql/server";
import { SEND_VERIFICATION_CODE, VERIFY_CODE } from "../graphql/mutations";

const AUTH_COOKIE_NAME = "auth_token";
const AUTH_COOKIE_MAX_AGE = 60 * 60 * 24 * 7; // 7 days

interface SendCodeResult {
  success: boolean;
  error?: string;
}

interface VerifyCodeResult {
  success: boolean;
  error?: string;
}

/**
 * Server action to send verification code
 */
export async function sendVerificationCode(phoneNumber: string): Promise<SendCodeResult> {
  try {
    await graphqlFetch<{ sendVerificationCode: boolean }>(
      SEND_VERIFICATION_CODE,
      { phoneNumber },
      { cache: "no-store" }
    );
    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : "Failed to send verification code",
    };
  }
}

/**
 * Server action to verify code and set auth cookie
 */
export async function verifyCode(phoneNumber: string, code: string): Promise<VerifyCodeResult> {
  try {
    const data = await graphqlFetch<{ verifyCode: string }>(
      VERIFY_CODE,
      { phoneNumber, code },
      { cache: "no-store" }
    );

    const token = data.verifyCode;

    // Set HTTP-only cookie with the auth token
    const cookieStore = await cookies();
    cookieStore.set(AUTH_COOKIE_NAME, token, {
      httpOnly: true,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      maxAge: AUTH_COOKIE_MAX_AGE,
      path: "/",
    });

    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : "Failed to verify code",
    };
  }
}

/**
 * Server action to logout and clear auth cookie
 */
export async function logout(): Promise<void> {
  const cookieStore = await cookies();
  cookieStore.delete(AUTH_COOKIE_NAME);
  redirect("/admin/login");
}

/**
 * Check if user is authenticated (for use in server components)
 */
export async function isAuthenticated(): Promise<boolean> {
  const cookieStore = await cookies();
  const token = cookieStore.get(AUTH_COOKIE_NAME)?.value;
  return !!token;
}

/**
 * Get the current auth token (for passing to graphql fetches)
 */
export async function getAuthToken(): Promise<string | null> {
  const cookieStore = await cookies();
  return cookieStore.get(AUTH_COOKIE_NAME)?.value ?? null;
}
