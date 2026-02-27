"use client";

import ReactMarkdown from "react-markdown";
import { markdownComponents } from "@/lib/markdown-components";

interface MarkdownPreviewProps {
  markdown: string;
  title?: string;
}

export function MarkdownPreview({ markdown, title }: MarkdownPreviewProps) {
  if (!markdown && !title) {
    return (
      <div className="flex items-center justify-center h-full text-text-faint text-sm">
        Start typing to see a preview...
      </div>
    );
  }

  return (
    <div className="p-8 max-w-prose mx-auto">
      {title && (
        <h1 className="text-2xl font-bold text-text-primary mb-6">{title}</h1>
      )}
      <div className="prose prose-stone max-w-none">
        <ReactMarkdown components={markdownComponents}>
          {markdown}
        </ReactMarkdown>
      </div>
    </div>
  );
}
