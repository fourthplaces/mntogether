"use client";

import { useState, useTransition } from "react";
import Link from "next/link";
import { callService } from "@/lib/restate/client";
import { Alert } from "@/components/ui/Alert";
import { BackLink } from "@/components/ui/BackLink";
import { Button } from "@/components/ui/Button";
import { Input, Textarea } from "@/components/ui/Input";
import { Card } from "@/components/ui/Card";
import type { SubmitResourceLinkResult } from "@/lib/restate/types";

export default function SubmitResourcePage() {
  const [url, setUrl] = useState("");
  const [context, setContext] = useState("");
  const [submitterContact, setSubmitterContact] = useState("");
  const [isPending, startTransition] = useTransition();
  const [result, setResult] = useState<SubmitResourceLinkResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setResult(null);

    startTransition(async () => {
      try {
        const data = await callService<SubmitResourceLinkResult>("Posts", "submit_resource_link", {
          url,
          context: context || null,
          submitter_contact: submitterContact || null,
        });

        setResult(data);
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
    <section className="max-w-[800px] mx-auto px-6 md:px-12 pt-10 pb-20">
      <BackLink href="/">Back to Home</BackLink>

      <h1 className="text-3xl font-bold text-text-primary leading-tight tracking-tight mb-2">Submit a Resource</h1>
      <p className="text-text-secondary mb-8">
        Share a link to an organization or resource that needs help
      </p>

      {result && (
        <Alert variant="success" className="mb-6">
          <p className="font-medium">Resource Submitted Successfully!</p>
          <p className="mt-1">{result.message}</p>
          <p className="mt-2 text-xs opacity-75">Job ID: {result.job_id}</p>
        </Alert>
      )}

      {error && (
        <Alert variant="error" className="mb-6">
          <p className="font-medium">Submission Failed</p>
          <p className="mt-1">{error}</p>
        </Alert>
      )}

      <Card>
        <form onSubmit={handleSubmit}>
          {/* URL Input */}
          <div className="mb-6">
            <label htmlFor="url" className="block text-sm font-medium text-text-secondary mb-2">
              Resource Link <span className="text-danger-text">*</span>
            </label>
            <Input
              type="url"
              id="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://example.com/resource"
              error={url && !isValidUrl(url) ? "Please enter a valid URL (must start with http:// or https://)" : undefined}
              required
            />
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
            <Textarea
              id="context"
              value={context}
              onChange={(e) => setContext(e.target.value)}
              placeholder="Tell us more about what this organization needs..."
              rows={4}
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
            <Input
              type="text"
              id="contact"
              value={submitterContact}
              onChange={(e) => setSubmitterContact(e.target.value)}
              placeholder="Email or phone number"
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
            <Button
              type="submit"
              variant="primary"
              disabled={!canSubmit || isPending}
              loading={isPending}
            >
              {isPending ? "Submitting..." : "Submit Resource"}
            </Button>
          </div>
        </form>
      </Card>

      {/* Info Section */}
      <Alert variant="info" className="mt-8">
        <h2 className="text-base font-semibold mb-3">What happens next?</h2>
        <ol className="space-y-2 text-sm">
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
      </Alert>
    </section>
  );
}
