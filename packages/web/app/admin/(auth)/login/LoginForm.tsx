"use client";

import { useState, useTransition } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { sendVerificationCode, verifyCode } from "@/lib/auth/actions";

export function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const rawRedirect = searchParams.get("redirect");
  const redirectUrl =
    rawRedirect && rawRedirect.startsWith("/admin/") && !rawRedirect.includes("//")
      ? rawRedirect
      : "/admin/dashboard";

  const [identifier, setIdentifier] = useState("");
  const [code, setCode] = useState("");
  const [step, setStep] = useState<"identifier" | "code">("identifier");
  const [error, setError] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  const handleSendCode = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!identifier.trim()) {
      setError("Please enter your phone number or email");
      return;
    }

    startTransition(async () => {
      const result = await sendVerificationCode(identifier);
      if (result.success) {
        setStep("code");
      } else {
        setError(result.error || "Failed to send verification code");
      }
    });
  };

  const handleVerifyCode = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!code.trim()) {
      setError("Please enter the verification code");
      return;
    }

    startTransition(async () => {
      const result = await verifyCode(identifier, code);
      if (result.success) {
        router.push(redirectUrl);
        router.refresh();
      } else {
        setError(result.error || "Failed to verify code");
      }
    });
  };

  return (
    <div className="bg-white rounded-lg shadow-md p-8 max-w-md w-full">
      <div className="mb-6 text-center">
        <h1 className="text-2xl font-bold text-gray-900 mb-2">Admin Login</h1>
        <p className="text-gray-600 text-sm">MN Together</p>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-800 rounded text-sm">
          {error}
        </div>
      )}

      {step === "identifier" ? (
        <form onSubmit={handleSendCode}>
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Phone Number or Email
            </label>
            <input
              type="text"
              value={identifier}
              onChange={(e) => setIdentifier(e.target.value)}
              placeholder="+1234567890 or admin@example.com"
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
              disabled={isPending}
            />
            <p className="mt-1 text-xs text-gray-500">
              Enter your registered phone number (with country code) or email address
            </p>
          </div>

          <button
            type="submit"
            disabled={isPending}
            className="w-full bg-amber-700 text-white py-2 px-4 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isPending ? "Sending..." : "Send Verification Code"}
          </button>
        </form>
      ) : (
        <form onSubmit={handleVerifyCode}>
          <div className="mb-4">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Verification Code
            </label>
            <input
              type="text"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder="Enter 6-digit code"
              className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
              disabled={isPending}
              autoFocus
            />
            <p className="mt-1 text-xs text-gray-500">
              Enter the verification code sent to {identifier}
            </p>
          </div>

          <div className="space-y-2">
            <button
              type="submit"
              disabled={isPending}
              className="w-full bg-amber-700 text-white py-2 px-4 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isPending ? "Verifying..." : "Verify & Sign In"}
            </button>

            <button
              type="button"
              onClick={() => {
                setStep("identifier");
                setCode("");
                setError(null);
              }}
              className="w-full bg-stone-100 text-stone-700 py-2 px-4 rounded-md hover:bg-stone-200 focus:outline-none focus:ring-2 focus:ring-stone-500 focus:ring-offset-2"
            >
              Back
            </button>
          </div>
        </form>
      )}
    </div>
  );
}
