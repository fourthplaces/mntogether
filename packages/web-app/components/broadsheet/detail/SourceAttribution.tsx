/**
 * SourceAttribution A — Centered footer rule.
 * Attribution · Source Name, centered below the article body.
 * Replaces raw footer-rule div.
 */
export function SourceAttributionA({ attribution, sourceName }: {
  attribution?: string | null;
  sourceName?: string | null;
}) {
  if (!attribution && !sourceName) return null;

  return (
    <div className="source-attribution-a">
      {attribution && <span>{attribution}</span>}
      {attribution && sourceName && <span className="source-attribution-a__sep"> &middot; </span>}
      {sourceName && <span>{sourceName}</span>}
    </div>
  );
}

/**
 * SourceAttribution B — Left-aligned byline style.
 * "Source: {name}" with attribution below.
 */
export function SourceAttributionB({ attribution, sourceName }: {
  attribution?: string | null;
  sourceName?: string | null;
}) {
  if (!attribution && !sourceName) return null;

  return (
    <div className="source-attribution-b">
      {sourceName && (
        <div className="source-attribution-b__source mono-sm">
          Source: {sourceName}
        </div>
      )}
      {attribution && (
        <div className="source-attribution-b__attribution">
          {attribution}
        </div>
      )}
    </div>
  );
}
