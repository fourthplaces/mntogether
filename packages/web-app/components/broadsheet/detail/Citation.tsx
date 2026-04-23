"use client";

import React from "react";
import ReactMarkdown from "react-markdown";

import {
  CitationIndex,
  parseCitations,
  resolveSignalUrl,
  type CitationSegment,
} from "@/lib/broadsheet/citations";

/**
 * A single `[1]`-style superscript marker. Renders as a link when a
 * signal-detail URL pattern is configured via
 * `NEXT_PUBLIC_SIGNAL_DETAIL_URL_PATTERN`; otherwise as an unlinked
 * superscript. The bracketed format is always included in the
 * rendered text so screen readers / text-copy behave sensibly.
 */
export function CitationMarker({
  uuid,
  index,
}: {
  uuid: string;
  index: number;
}) {
  const href = resolveSignalUrl(uuid);
  const label = `[${index}]`;
  const title = `Signal source ${uuid}`;

  if (!href) {
    return (
      <sup className="citation citation--unlinked" title={title} data-uuid={uuid}>
        {label}
      </sup>
    );
  }
  return (
    <sup className="citation" data-uuid={uuid}>
      <a href={href} target="_blank" rel="noopener noreferrer" title={title}>
        {label}
      </a>
    </sup>
  );
}

/**
 * Render inline text with `[signal:UUID]` tokens converted to
 * superscript citation markers. `index` must be a shared
 * `CitationIndex` instance for the whole body so numbering is stable.
 */
export function CitationText({
  text,
  index,
}: {
  text: string;
  index: CitationIndex;
}) {
  const segments = parseCitations(text, index);
  if (segments.length === 0) return null;
  return (
    <>
      {segments.map((s, i) => renderSegment(s, i))}
    </>
  );
}

function renderSegment(segment: CitationSegment, key: number): React.ReactNode {
  if (segment.kind === "text") return <React.Fragment key={key}>{segment.text}</React.Fragment>;
  return (
    <CitationMarker key={key} uuid={segment.uuid} index={segment.index} />
  );
}

/**
 * Wrap a markdown body, replacing inline `[signal:UUID]` tokens with
 * superscript markers. Uses react-markdown's `text` component hook so
 * the tokens survive inline markdown (bold, italic, links) in the
 * surrounding text without being consumed as markdown brackets.
 */
export function CitationMarkdown({
  source,
  index,
}: {
  source: string;
  index: CitationIndex;
}) {
  return (
    <ReactMarkdown
      components={{
        a: ({ href, children }) => (
          <a href={href} target="_blank" rel="noopener noreferrer">
            {children}
          </a>
        ),
        h1: ({ children }) => <h2>{children}</h2>,
        h2: ({ children }) => <h3>{children}</h3>,
        h3: ({ children }) => <h4>{children}</h4>,
        // The `text` component fires for every leaf text node. Parse
        // each one and interleave citation markers. React-markdown
        // passes a string via `children`.
        text: ({ children }) => {
          const value = typeof children === "string" ? children : String(children ?? "");
          if (!value.includes("[signal:")) return <>{value}</>;
          return <CitationText text={value} index={index} />;
        },
      }}
    >
      {source}
    </ReactMarkdown>
  );
}
