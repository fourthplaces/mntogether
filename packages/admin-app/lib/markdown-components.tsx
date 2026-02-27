// Shared ReactMarkdown component overrides for consistent rendering
// Used by MarkdownPreview (editor right pane) and post detail page

import type { Components } from "react-markdown";

export const markdownComponents: Components = {
  p: ({ children }) => <p className="mb-4 text-stone-700">{children}</p>,
  ul: ({ children }) => (
    <ul className="list-disc pl-6 mb-4 space-y-1">{children}</ul>
  ),
  ol: ({ children }) => (
    <ol className="list-decimal pl-6 mb-4 space-y-1">{children}</ol>
  ),
  li: ({ children }) => <li className="text-stone-700">{children}</li>,
  strong: ({ children }) => (
    <strong className="font-semibold">{children}</strong>
  ),
  a: ({ href, children }) => (
    <a
      href={href}
      className="text-blue-600 hover:text-blue-800 underline"
      target="_blank"
      rel="noopener noreferrer"
    >
      {children}
    </a>
  ),
  h1: ({ children }) => (
    <h1 className="text-2xl font-bold text-stone-900 mb-4 mt-6">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="text-xl font-semibold text-stone-900 mb-3 mt-5">{children}</h2>
  ),
  h3: ({ children }) => (
    <h3 className="text-lg font-semibold text-stone-900 mb-2 mt-4">{children}</h3>
  ),
  blockquote: ({ children }) => (
    <blockquote className="border-l-3 border-admin-accent pl-4 italic text-stone-600 mb-4">
      {children}
    </blockquote>
  ),
  code: ({ children, className }) => {
    // Inline code vs code blocks
    if (className) {
      return (
        <code className="block bg-surface-muted text-text-primary font-mono text-sm p-4 rounded-md border border-border mb-4 overflow-x-auto">
          {children}
        </code>
      );
    }
    return (
      <code className="bg-surface-muted text-text-primary font-mono text-sm px-1.5 py-0.5 rounded">
        {children}
      </code>
    );
  },
  hr: () => <hr className="border-border my-6" />,
};
