"use client";

import { useState, useEffect, useTransition } from "react";
import { BottomSheet } from "@/components/public/BottomSheet";
import { callService } from "@/lib/restate/client";
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
          <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg">
            <p className="text-sm font-medium text-green-800">Submitted!</p>
            <p className="text-sm text-green-700 mt-0.5">{result.message}</p>
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
            disabled={!canSubmit || isPending}
            className="mt-3 w-full py-2.5 bg-blue-600 text-white text-sm font-medium rounded-xl hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isPending ? "Submitting..." : "Submit"}
          </button>
        </form>
      </div>
    </BottomSheet>
  );
}
