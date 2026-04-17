"use client";

import * as React from "react";
import Link from "next/link";
import ReactMarkdown from "react-markdown";
import { Button } from "@/components/ui/button";
import { Pencil } from "lucide-react";
import { markdownComponents } from "@/lib/markdown-components";
import { BodyPreview } from "./BodyPreview";
import { HeroPhotoEditor } from "./HeroPhotoEditor";

type LeftPost = {
  id: string;
  bodyRaw?: string | null;
  bodyHeavy?: string | null;
  bodyMedium?: string | null;
  bodyLight?: string | null;
  media?: Array<{ imageUrl?: string | null; caption?: string | null; credit?: string | null; mediaId?: string | null }> | null;
};

export function PostDetailLeft({
  post,
  postId,
  onSaveMedia,
}: {
  post: LeftPost;
  postId: string;
  onSaveMedia: (input: { imageUrl: string | null; caption: string | null; credit: string | null; mediaId: string | null }) => Promise<unknown>;
}) {
  const heroMedia = post.media && post.media.length > 0 ? post.media[0] : null;

  return (
    <div className="space-y-6">
      {/* Edit body action */}
      <div className="flex items-center justify-between">
        <h2 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Content
        </h2>
        <Button render={<Link href={`/admin/posts/${postId}/edit`} />} variant="outline" size="sm">
          <Pencil className="w-3.5 h-3.5 mr-1.5" />
          Edit body
        </Button>
      </div>

      {/* Hero photo */}
      <HeroPhotoEditor media={heroMedia} onSave={onSaveMedia} />

      {/* Full text */}
      <div>
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-3">
          Full Text
        </h3>
        <div className="prose prose-stone max-w-none text-sm">
          {post.bodyRaw ? (
            <ReactMarkdown components={markdownComponents}>{post.bodyRaw}</ReactMarkdown>
          ) : (
            <p className="text-sm text-muted-foreground italic">No body text yet</p>
          )}
        </div>
      </div>

      {/* Body variants — Heavy / Medium / Light */}
      <div className="space-y-4">
        <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Body Variants
        </h3>
        <BodyPreview label="Heavy" text={post.bodyHeavy} />
        <BodyPreview label="Medium" text={post.bodyMedium} />
        <BodyPreview label="Light" text={post.bodyLight} />
      </div>
    </div>
  );
}
