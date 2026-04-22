"use client";

import { useQuery } from "urql";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { AdminLoader } from "./AdminLoader";
import { SignalInboxCanonicalQuery } from "@/lib/graphql/signal-inbox";

interface MergeCandidate {
  id: string;
  title: string;
  bodyRaw: string;
  bodyLight?: string | null;
  postType?: string | null;
  weight?: string | null;
  location?: string | null;
  publishedAt?: string | null;
  organizationName?: string | null;
  sourceUrl?: string | null;
}

interface MergeDuplicateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  incoming: MergeCandidate;
  canonicalId: string;
  onConfirmReject: () => void;
  busy?: boolean;
}

function truncate(text: string, n: number): string {
  if (!text) return "";
  return text.length > n ? `${text.slice(0, n).trim()}…` : text;
}

function SideColumn({
  heading,
  post,
  badge,
}: {
  heading: string;
  post: MergeCandidate | null | undefined;
  badge: string;
}) {
  return (
    <div className="flex-1 min-w-0 border border-border rounded-lg p-4 bg-card">
      <div className="flex items-center gap-2 mb-2">
        <span className="text-xs uppercase tracking-wider font-medium text-muted-foreground">
          {heading}
        </span>
        <span className="text-[10px] px-1.5 py-0.5 bg-muted text-muted-foreground rounded-full font-mono">
          {badge}
        </span>
      </div>
      {!post ? (
        <p className="text-sm text-muted-foreground italic">No canonical post found.</p>
      ) : (
        <>
          <div className="text-base font-semibold text-foreground leading-tight mb-1">
            {post.title}
          </div>
          <div className="text-xs text-muted-foreground mb-2">
            {[post.postType, post.weight, post.location].filter(Boolean).join(" · ")}
          </div>
          {post.organizationName && (
            <div className="text-xs mb-2">
              <span className="text-muted-foreground">Org: </span>
              {post.organizationName}
            </div>
          )}
          {post.sourceUrl && (
            <div className="text-xs mb-2 break-all">
              <span className="text-muted-foreground">Source: </span>
              <a
                href={post.sourceUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-admin-accent hover:underline"
              >
                {post.sourceUrl}
              </a>
            </div>
          )}
          <p className="text-sm text-muted-foreground line-clamp-6">
            {post.bodyLight?.trim() || truncate(post.bodyRaw || "", 500)}
          </p>
        </>
      )}
    </div>
  );
}

export function MergeDuplicateDialog({
  open,
  onOpenChange,
  incoming,
  canonicalId,
  onConfirmReject,
  busy,
}: MergeDuplicateDialogProps) {
  const [{ data, fetching }] = useQuery({
    query: SignalInboxCanonicalQuery,
    variables: { id: canonicalId },
    pause: !open,
  });

  const canonical = data?.post ?? null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>Review duplicate merge</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          Root Signal flagged the incoming post as a duplicate of an existing canonical post.
          Confirm to archive the incoming one; open the canonical to merge in any new fields first.
        </p>

        {fetching ? (
          <AdminLoader label="Loading canonical post..." />
        ) : (
          <div className="flex gap-3 items-stretch">
            <SideColumn heading="Incoming (in review)" post={incoming} badge="new" />
            <SideColumn heading="Canonical (active)" post={canonical} badge="kept" />
          </div>
        )}

        <div className="flex justify-between items-center gap-3 mt-2">
          <Button
            variant="outline"
            disabled={!canonical}
            render={<Link href={`/admin/posts/${canonicalId}`} />}
          >
            Open canonical to edit fields
          </Button>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={onConfirmReject}
              disabled={busy}
            >
              Archive incoming as duplicate
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
