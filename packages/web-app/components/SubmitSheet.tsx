"use client";

import { useState, useEffect } from "react";
import { useMutation } from "urql";
import { BottomSheet } from "@/components/BottomSheet";
import { SubmitResourceLinkMutation } from "@/lib/graphql/posts";

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
          <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg">
            <p className="text-sm font-medium text-green-800">Submitted!</p>
            <p className="text-sm text-green-700 mt-0.5">Thanks for sharing! We'll review it shortly.</p>
          </div>
        )}

        {error && (
          <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg">
            <p className="text-sm font-medium text-red-800">Error</p>
            <p className="text-sm text-red-700 mt-0.5">{error}</p>
          </div>
        )}

        <form onSubmit={handleSubmit}>
          <p className="text-sm text-gray-500 mb-3">
            Know someone who needs help, a place to volunteer, or a community update? Paste the link and we'll add it to the directory.
          </p>
          <label htmlFor="submit-url" className="block text-sm font-medium text-gray-700 mb-1.5">
            Link
          </label>
          <input
            id="submit-url"
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://..."
            className="w-full px-4 py-2.5 border border-gray-300 rounded-xl text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          />
          {url && !isValidUrl(url) && (
            <p className="mt-1 text-xs text-red-600">Enter a valid URL</p>
          )}

          <button
            type="submit"
            disabled={!canSubmit || fetching}
            className="mt-3 w-full py-2.5 bg-blue-600 text-white text-sm font-medium rounded-xl hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {fetching ? "Submitting..." : "Submit"}
          </button>
        </form>
      </div>
    </BottomSheet>
  );
}
