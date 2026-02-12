"use client";

import { useState, useEffect, useTransition } from "react";
import { BottomSheet } from "@/components/public/BottomSheet";
import { callService } from "@/lib/restate/client";
import { Alert } from "@/components/ui/Alert";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import type { SubmitResourceLinkResult } from "@/lib/restate/types";

interface SubmitSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SubmitSheet({ isOpen, onClose }: SubmitSheetProps) {
  const [url, setUrl] = useState("");
  const [isPending, startTransition] = useTransition();
  const [result, setResult] = useState<SubmitResourceLinkResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Reset form state when sheet closes
  useEffect(() => {
    if (!isOpen) {
      setUrl("");
      setResult(null);
      setError(null);
    }
  }, [isOpen]);

  const isValidUrl = (s: string) => {
    try {
      new URL(s);
      return true;
    } catch {
      return false;
    }
  };

  const canSubmit = url.trim() !== "" && isValidUrl(url);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;

    setError(null);
    setResult(null);

    startTransition(async () => {
      try {
        const data = await callService<SubmitResourceLinkResult>(
          "Posts",
          "submit_resource_link",
          { url }
        );
        setResult(data);
        setUrl("");
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to submit");
      }
    });
  };

  return (
    <BottomSheet isOpen={isOpen} onClose={onClose} title="Share a Community Resource">
      <div className="px-4 pb-6">
        {result && (
          <Alert variant="success" className="mb-4">
            <p className="font-medium">Submitted!</p>
            <p className="mt-0.5">{result.message}</p>
          </Alert>
        )}

        {error && (
          <Alert variant="error" className="mb-4">
            <p className="font-medium">Error</p>
            <p className="mt-0.5">{error}</p>
          </Alert>
        )}

        <form onSubmit={handleSubmit}>
          <p className="text-sm text-text-muted mb-3">
            Know someone who needs help, a place to volunteer, or a community update? Paste the link and we'll add it to the directory.
          </p>
          <label htmlFor="submit-url" className="block text-sm font-medium text-text-secondary mb-1.5">
            Link
          </label>
          <Input
            id="submit-url"
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://..."
            error={url && !isValidUrl(url) ? "Enter a valid URL" : undefined}
          />

          <Button
            type="submit"
            variant="primary"
            className="mt-3 w-full"
            disabled={!canSubmit || isPending}
            loading={isPending}
          >
            {isPending ? "Submitting..." : "Submit"}
          </Button>
        </form>
      </div>
    </BottomSheet>
  );
}
