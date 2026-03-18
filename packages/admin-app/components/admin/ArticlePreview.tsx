"use client";

/**
 * ArticlePreview — live preview of the article body using web-app styling.
 *
 * Renders the left-column content of the broadsheet post detail page:
 * kicker → title (with deck) → media → body → person pull quote.
 *
 * Scoped under .article-preview to prevent CSS leaking into admin UI.
 * Uses article-preview.css (subset of web-app broadsheet-detail.css).
 */

import "@/app/article-preview.css";
import ReactMarkdown from "react-markdown";

interface ArticlePreviewProps {
  title: string;
  markdown: string;
  postType?: string;
  // Meta fields
  kicker?: string;
  byline?: string;
  deck?: string;
  // Media fields
  imageUrl?: string;
  caption?: string;
  credit?: string;
  // Person fields (spotlight)
  personName?: string;
  personRole?: string;
  personQuote?: string;
}

export function ArticlePreview({
  title,
  markdown,
  postType = "story",
  kicker,
  byline,
  deck,
  imageUrl,
  caption,
  credit,
  personName,
  personRole,
  personQuote,
}: ArticlePreviewProps) {
  const hasKicker = !!kicker;
  const hasMeta = !!byline;
  const hasMedia = !!imageUrl;
  const hasQuote = !!personQuote;
  const hasBody = markdown.trim().length > 0;

  return (
    <div className="article-preview">
      {/* Kicker */}
      {hasKicker && <div className="kicker">{kicker}</div>}

      {/* Title */}
      <h1 className={`preview-title preview-title--${postType}`}>
        {title || <span className="preview-empty">Untitled</span>}
      </h1>

      {/* Deck */}
      {deck && <div className="preview-deck">{deck}</div>}

      {/* Meta (byline + date) */}
      {hasMeta && (
        <div className="preview-meta">
          {byline && <span>{byline}</span>}
        </div>
      )}

      {/* Photo */}
      {hasMedia && (
        <div className="preview-photo">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={imageUrl} alt={caption || title} />
          {(caption || credit) && (
            <div className="preview-photo__caption">
              {caption && (
                <span className="preview-photo__caption-text">{caption}</span>
              )}
              {credit && (
                <span className="preview-photo__credit">{credit}</span>
              )}
            </div>
          )}
        </div>
      )}

      {/* Body */}
      {hasBody ? (
        <div className="preview-body">
          <ReactMarkdown>{markdown}</ReactMarkdown>
        </div>
      ) : (
        <p className="preview-empty">Start writing to see a preview...</p>
      )}

      {/* Pull quote (from person) */}
      {hasQuote && (
        <div className="preview-pull-quote">
          &ldquo;{personQuote}&rdquo;
          {(personName || personRole) && (
            <div className="preview-pull-quote__attribution">
              — {[personName, personRole].filter(Boolean).join(", ")}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
