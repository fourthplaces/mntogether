"use client";

import { useState, useTransition } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { sendVerificationCode, verifyCode } from "@/lib/auth/actions";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Alert } from "@/components/ui/alert";

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
    <div className="bg-card rounded-lg shadow-md p-8 max-w-md w-full">
      <div className="mb-6 text-center">
        <h1 className="text-2xl font-bold text-foreground mb-2">Admin Login</h1>
        <p className="text-muted-foreground text-sm">MN Together</p>
      </div>

      {error && (
        <Alert variant="warning" className="mb-4">
          {error}
        </Alert>
      )}

      {step === "identifier" ? (
        <form onSubmit={handleSendCode}>
          <div className="mb-4">
            <label className="block text-sm font-medium text-foreground mb-2">
              Phone Number or Email
            </label>
            <Input
              type="text"
              value={identifier}
              onChange={(e) => setIdentifier(e.target.value)}
              placeholder="+1234567890 or admin@example.com"
              disabled={isPending}
            />
            <p className="mt-1 text-xs text-muted-foreground">
              Enter your registered phone number (with country code) or email address
            </p>
          </div>

          <Button
            type="submit"
            variant="admin"
            disabled={isPending}
            loading={isPending}
            className="w-full"
          >
            {isPending ? "Sending..." : "Send Verification Code"}
          </Button>
        </form>
      ) : (
        <form onSubmit={handleVerifyCode}>
          <div className="mb-4">
            <label className="block text-sm font-medium text-foreground mb-2">
              Verification Code
            </label>
            <Input
              type="text"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder="Enter 6-digit code"
              disabled={isPending}
              autoFocus
            />
            <p className="mt-1 text-xs text-muted-foreground">
              Enter the verification code sent to {identifier}
            </p>
          </div>

          <div className="space-y-2">
            <Button
              type="submit"
              variant="admin"
              disabled={isPending}
              loading={isPending}
              className="w-full"
            >
              {isPending ? "Verifying..." : "Verify & Sign In"}
            </Button>

            <Button
              type="button"
              variant="secondary"
              className="w-full"
              onClick={() => {
                setStep("identifier");
                setCode("");
                setError(null);
              }}
            >
              Back
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}
