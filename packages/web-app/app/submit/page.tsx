"use client";

import { useState } from "react";
import Link from "next/link";
import { useMutation } from "urql";
import { SubmitResourceLinkMutation } from "@/lib/graphql/public";

export default function SubmitResourcePage() {
  const [url, setUrl] = useState("");
  const [context, setContext] = useState("");
  const [submitterContact, setSubmitterContact] = useState("");
  const [resultMessage, setResultMessage] = useState<string | null>(null);
  const [resultJobId, setResultJobId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [{ fetching }, submitResource] = useMutation(SubmitResourceLinkMutation);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setResultMessage(null);
    setResultJobId(null);

    try {
      const result = await submitResource({
        url,
        context: context || null,
        submitterContact: submitterContact || null,
      });

      if (result.error) throw result.error;

      if (result.data?.submitResourceLink) {
        setResultMessage(result.data.submitResourceLink.message);
        setResultJobId(result.data.submitResourceLink.jobId ?? null);
      }
      // Reset form after successful submission
      setUrl("");
      setContext("");
      setSubmitterContact("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit resource");
    }
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
    <section className="max-w-[800px] mx-auto px-6 md:px-12 pt-10 pb-20">
      <Link href="/" className="inline-block text-sm text-text-secondary hover:text-text-primary mb-6">
        &larr; Back to Home
      </Link>

      <h1 className="text-3xl font-bold text-text-primary leading-tight tracking-tight mb-2">Submit a Resource</h1>
      <p className="text-text-secondary mb-8">
        Share a link to an organization or resource that needs help
      </p>

      {resultMessage && (
        <div className="mb-6 p-4 bg-success-bg border border-success">
          <h3 className="text-sm font-medium text-success-text">Resource Submitted Successfully!</h3>
          <p className="mt-1 text-sm text-success-text">{resultMessage}</p>
          {resultJobId && <p className="mt-2 text-xs text-success-text opacity-75">Job ID: {resultJobId}</p>}
        </div>
      )}

      {error && (
        <div className="mb-6 p-4 bg-danger-bg border border-danger">
          <p className="text-sm font-medium text-danger-text">Submission Failed</p>
          <p className="mt-1 text-sm text-danger-text">{error}</p>
        </div>
      )}

      <div className="bg-surface-raised border border-border p-6">
        <form onSubmit={handleSubmit}>
          {/* URL Input */}
          <div className="mb-6">
            <label htmlFor="url" className="block text-sm font-medium text-text-secondary mb-2">
              Resource Link <span className="text-danger-text">*</span>
            </label>
            <input
              type="url"
              id="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://example.com/resource"
              required
              className="w-full px-4 py-2 border border-border text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-action"
            />
            {url && !isValidUrl(url) && (
              <p className="text-xs text-danger-text mt-1">Please enter a valid URL (must start with http:// or https://)</p>
            )}
            <p className="mt-2 text-sm text-text-muted">
              Paste a link to an organization&apos;s website, social media post, or any resource
              that needs attention
            </p>
          </div>

          {/* Context Input */}
          <div className="mb-6">
            <label htmlFor="context" className="block text-sm font-medium text-text-secondary mb-2">
              Context (Optional)
            </label>
            <textarea
              id="context"
              value={context}
              onChange={(e) => setContext(e.target.value)}
              placeholder="Tell us more about what this organization needs..."
              rows={4}
              className="w-full px-4 py-2 border border-border text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-action resize-none"
            />
            <p className="mt-2 text-sm text-text-muted">
              Provide any additional context that might help us understand what they need
            </p>
          </div>

          {/* Contact Input */}
          <div className="mb-6">
            <label htmlFor="contact" className="block text-sm font-medium text-text-secondary mb-2">
              Your Contact (Optional)
            </label>
            <input
              type="text"
              id="contact"
              value={submitterContact}
              onChange={(e) => setSubmitterContact(e.target.value)}
              placeholder="Email or phone number"
              className="w-full px-4 py-2 border border-border text-sm text-text-primary placeholder-text-muted focus:outline-none focus:border-action"
            />
            <p className="mt-2 text-sm text-text-muted">
              Optional: Leave your contact info if you&apos;d like us to follow up with you
            </p>
          </div>

          {/* Submit Button */}
          <div className="flex items-center justify-between">
            <Link href="/" className="text-sm text-text-secondary hover:text-text-primary">
              Cancel
            </Link>
            <button
              type="submit"
              disabled={!canSubmit || fetching}
              className="px-6 py-2 bg-action text-text-on-action text-sm font-semibold hover:bg-action-hover disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {fetching ? "Submitting..." : "Submit Resource"}
            </button>
          </div>
        </form>
      </div>

      {/* Info Section */}
      <div className="mt-8 bg-info-bg border border-border p-6">
        <h2 className="text-base font-semibold text-info-text mb-3">What happens next?</h2>
        <ol className="space-y-2 text-sm text-info-text">
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
    </section>
  );
}
