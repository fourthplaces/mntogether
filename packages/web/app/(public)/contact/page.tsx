import Link from "next/link";
import { BackLink } from "@/components/ui/BackLink";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";

export default function ContactPage() {
  return (
    <section className="max-w-[800px] mx-auto px-6 md:px-12 pt-10 pb-20">
      <BackLink href="/">Back to Home</BackLink>

      <h1 className="text-3xl font-bold text-text-primary leading-tight tracking-tight mb-8">Contact Us</h1>

      <div className="space-y-6 text-text-body text-base leading-relaxed">
        <p>
          Have a question, want to submit a resource, or interested in partnering
          with MN Together? We&apos;d love to hear from you.
        </p>

        <Card padding="lg" className="space-y-6">
          <div>
            <h2 className="text-lg font-semibold text-text-primary mb-2">Email</h2>
            <a
              href="mailto:hello@mntogether.org"
              className="text-text-secondary hover:text-text-primary underline"
            >
              hello@mntogether.org
            </a>
          </div>

          <div>
            <h2 className="text-lg font-semibold text-text-primary mb-2">Submit a Resource</h2>
            <p className="text-text-secondary mb-3">
              Know of a service, organization, or event that should be listed?
              Let us know and we&apos;ll review it for inclusion.
            </p>
            <Button
              variant="primary"
              pill
              href="mailto:hello@mntogether.org?subject=Resource Submission"
            >
              Submit a Resource
            </Button>
          </div>

          <div>
            <h2 className="text-lg font-semibold text-text-primary mb-2">Partnerships</h2>
            <p className="text-text-secondary">
              If you represent an organization and want to reach more people in
              Minneapolis, reach out to discuss how we can work together.
            </p>
          </div>
        </Card>
      </div>
    </section>
  );
}
