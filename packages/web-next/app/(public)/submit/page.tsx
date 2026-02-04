"use client";

import { useState, useTransition } from "react";
import Link from "next/link";
import { graphqlMutateClient } from "@/lib/graphql/client";
import { SUBMIT_RESOURCE_LINK } from "@/lib/graphql/mutations";
import type { SubmitResourceLinkResult } from "@/lib/types";

export default function SubmitResourcePage() {
  const [url, setUrl] = useState("");
  const [context, setContext] = useState("");
  const [submitterContact, setSubmitterContact] = useState("");
  const [isPending, startTransition] = useTransition();
  const [result, setResult] = useState<SubmitResourceLinkResult["submitResourceLink"] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setResult(null);

    startTransition(async () => {
      try {
        const data = await graphqlMutateClient<SubmitResourceLinkResult>(SUBMIT_RESOURCE_LINK, {
          input: {
            url,
            context: context || null,
            submitterContact: submitterContact || null,
          },
        });

        setResult(data.submitResourceLink);
        // Reset form after successful submission
        setUrl("");
        setContext("");
        setSubmitterContact("");
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to submit resource");
      }
    });
  };

  const isValidUrl = (urlString: string) => {
    try {
      new URL(urlString);
      return true;
    } catch {
      return false;
    }
  };

  const canSubmit = url.trim() !== "" && isValidUrl(url);

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-white border-b border-gray-200">
        <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <Link href="/" className="text-blue-600 hover:text-blue-800 text-sm mb-2 inline-block">
            &larr; Back to Home
          </Link>
          <h1 className="text-3xl font-bold text-gray-900">Submit a Resource</h1>
          <p className="mt-2 text-gray-600">
            Share a link to an organization or resource that needs help
          </p>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {result && (
          <div className="mb-6 p-4 bg-green-50 border border-green-200 rounded-lg">
            <div className="flex items-start">
              <svg
                className="w-5 h-5 text-green-600 mt-0.5 mr-3 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              <div>
                <h3 className="text-sm font-medium text-green-800">
                  Resource Submitted Successfully!
                </h3>
                <p className="mt-1 text-sm text-green-700">{result.message}</p>
                <p className="mt-2 text-xs text-green-600">Job ID: {result.jobId}</p>
              </div>
            </div>
          </div>
        )}

        {error && (
          <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg">
            <div className="flex items-start">
              <svg
                className="w-5 h-5 text-red-600 mt-0.5 mr-3 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                  clipRule="evenodd"
                />
              </svg>
              <div>
                <h3 className="text-sm font-medium text-red-800">Submission Failed</h3>
                <p className="mt-1 text-sm text-red-700">{error}</p>
              </div>
            </div>
          </div>
        )}

        <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
          <form onSubmit={handleSubmit}>
            {/* URL Input */}
            <div className="mb-6">
              <label htmlFor="url" className="block text-sm font-medium text-gray-700 mb-2">
                Resource Link <span className="text-red-500">*</span>
              </label>
              <input
                type="url"
                id="url"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://example.com/resource"
                className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                required
              />
              <p className="mt-2 text-sm text-gray-500">
                Paste a link to an organization&apos;s website, social media post, or any resource
                that needs attention
              </p>
              {url && !isValidUrl(url) && (
                <p className="mt-1 text-sm text-red-600">
                  Please enter a valid URL (must start with http:// or https://)
                </p>
              )}
            </div>

            {/* Context Input */}
            <div className="mb-6">
              <label htmlFor="context" className="block text-sm font-medium text-gray-700 mb-2">
                Context (Optional)
              </label>
              <textarea
                id="context"
                value={context}
                onChange={(e) => setContext(e.target.value)}
                placeholder="Tell us more about what this organization needs..."
                rows={4}
                className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <p className="mt-2 text-sm text-gray-500">
                Provide any additional context that might help us understand what they need
              </p>
            </div>

            {/* Contact Input */}
            <div className="mb-6">
              <label htmlFor="contact" className="block text-sm font-medium text-gray-700 mb-2">
                Your Contact (Optional)
              </label>
              <input
                type="text"
                id="contact"
                value={submitterContact}
                onChange={(e) => setSubmitterContact(e.target.value)}
                placeholder="Email or phone number"
                className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
              />
              <p className="mt-2 text-sm text-gray-500">
                Optional: Leave your contact info if you&apos;d like us to follow up with you
              </p>
            </div>

            {/* Submit Button */}
            <div className="flex items-center justify-between">
              <Link href="/" className="text-sm text-gray-600 hover:text-gray-900">
                Cancel
              </Link>
              <button
                type="submit"
                disabled={!canSubmit || isPending}
                className="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                {isPending ? "Submitting..." : "Submit Resource"}
              </button>
            </div>
          </form>
        </div>

        {/* Info Section */}
        <div className="mt-8 bg-blue-50 border border-blue-200 rounded-lg p-6">
          <h2 className="text-lg font-semibold text-blue-900 mb-3">What happens next?</h2>
          <ol className="space-y-2 text-sm text-blue-800">
            <li className="flex items-start">
              <span className="font-medium mr-2">1.</span>
              <span>We&apos;ll automatically scrape the link you provided</span>
            </li>
            <li className="flex items-start">
              <span className="font-medium mr-2">2.</span>
              <span>Our AI will extract any needs or opportunities from the content</span>
            </li>
            <li className="flex items-start">
              <span className="font-medium mr-2">3.</span>
              <span>An admin will review and approve the needs</span>
            </li>
            <li className="flex items-start">
              <span className="font-medium mr-2">4.</span>
              <span>Approved needs will appear on the home page for people to help</span>
            </li>
          </ol>
        </div>
      </main>
    </div>
  );
}
