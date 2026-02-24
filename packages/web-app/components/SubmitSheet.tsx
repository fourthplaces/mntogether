"use client";

import { useState, useEffect } from "react";
import { useMutation } from "urql";
import { BottomSheet } from "@/components/BottomSheet";
import { SubmitResourceLinkMutation } from "@/lib/graphql/public";

interface SubmitSheetProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SubmitSheet({ isOpen, onClose }: SubmitSheetProps) {
  const [url, setUrl] = useState("");
  const [submitted, setSubmitted] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [{ fetching }, submitResource] = useMutation(SubmitResourceLinkMutation);

  // Reset form state when sheet closes
  useEffect(() => {
    if (!isOpen) {
      setUrl("");
      setSubmitted(false);
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

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;

    setError(null);
    setSubmitted(false);

    try {
      const result = await submitResource({ url });
      if (result.error) throw result.error;
      setSubmitted(true);
      setUrl("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit");
    }
  };

  return (
    <BottomSheet isOpen={isOpen} onClose={onClose} title="Share a Community Resource">
      <div className="px-4 pb-6">
        {submitted && (
          <div className="mb-4 p-3 bg-success-bg border border-success text-sm">
            <p className="font-medium text-success-text">Submitted!</p>
            <p className="text-success-text mt-0.5">Thanks for sharing! We&apos;ll review it shortly.</p>
          </div>
        )}

        {error && (
          <div className="mb-4 p-3 bg-danger-bg border border-danger text-sm">
            <p className="font-medium text-danger-text">Error</p>
            <p className="text-danger-text mt-0.5">{error}</p>
          </div>
        )}

        <form onSubmit={handleSubmit}>
          <p className="text-sm text-text-muted mb-3">
            Know someone who needs help, a place to volunteer, or a community update? Paste the link and we&apos;ll add it to the directory.
          </p>
          <label htmlFor="submit-url" className="block text-sm font-medium text-text-secondary mb-1.5">
            Link
          </label>
          <input
            id="submit-url"
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://..."
            className="w-full px-4 py-2 border border-border text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-action"
          />
          {url && !isValidUrl(url) && (
            <p className="text-xs text-danger-text mt-1">Enter a valid URL</p>
          )}

          <button
            type="submit"
            disabled={!canSubmit || fetching}
            className="mt-3 w-full py-2.5 bg-action text-text-on-action text-sm font-medium hover:bg-action-hover disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {fetching ? "Submitting..." : "Submit"}
          </button>
        </form>
      </div>
    </BottomSheet>
  );
}
