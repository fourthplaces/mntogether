/**
 * Citation parsing + numbering for Root Signal `[signal:UUID]` tokens.
 *
 * Signal emits body copy with inline tokens of the form `[signal:UUID]`
 * marking where a sentence/claim was sourced from a specific Signal
 * record. Editorial parses these at render time and presents them as
 * superscript citations (`[1]`, `[2]`, …) linked to Signal's
 * signal-detail URL. See
 * `docs/handoff-root-signal/ROOT_SIGNAL_API_REQUEST.md` §15.2.
 *
 * Numbering is per-render-scope: the first occurrence of a given UUID
 * gets `[1]`, the second distinct UUID `[2]`, etc. Repeat references to
 * the same UUID reuse the same number. A single `CitationIndex`
 * instance should span a whole body render so numbers are stable
 * across paragraphs / AST nodes.
 */

export const SIGNAL_CITATION_RE = /\[signal:([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\]/gi;

export type CitationSegment =
  | { kind: "text"; text: string }
  | { kind: "citation"; uuid: string; index: number };

/**
 * Running registry that assigns a stable 1-based index to each unique
 * UUID in the order first seen. Pass a single instance through the
 * whole body render so the same UUID always renders as the same number.
 */
export class CitationIndex {
  private map = new Map<string, number>();
  private order: string[] = [];

  /** Get or assign an index for this UUID. */
  indexFor(uuid: string): number {
    const normalized = uuid.toLowerCase();
    const existing = this.map.get(normalized);
    if (existing !== undefined) return existing;
    const next = this.order.length + 1;
    this.map.set(normalized, next);
    this.order.push(normalized);
    return next;
  }

  /** UUIDs in first-seen order. */
  uuids(): string[] {
    return [...this.order];
  }

  hasAny(): boolean {
    return this.order.length > 0;
  }
}

/**
 * Split a string into an alternating sequence of text and citation
 * segments. Text segments with empty content are dropped.
 */
export function parseCitations(
  input: string,
  index: CitationIndex,
): CitationSegment[] {
  if (!input) return [];
  const segments: CitationSegment[] = [];
  let lastEnd = 0;
  // Fresh RegExp — the module-level constant is stateful across calls
  // due to the /g flag; we don't want prior consumers to advance
  // lastIndex on shared state.
  const re = new RegExp(SIGNAL_CITATION_RE.source, "gi");
  let m: RegExpExecArray | null;
  while ((m = re.exec(input)) !== null) {
    const [full, uuid] = m;
    if (m.index > lastEnd) {
      segments.push({ kind: "text", text: input.slice(lastEnd, m.index) });
    }
    segments.push({ kind: "citation", uuid, index: index.indexFor(uuid) });
    lastEnd = m.index + full.length;
  }
  if (lastEnd < input.length) {
    segments.push({ kind: "text", text: input.slice(lastEnd) });
  }
  return segments;
}

/**
 * Strip all `[signal:UUID]` tokens from a string. Used where plain
 * text is needed (og tags, meta descriptions, search snippets).
 */
export function stripCitations(input: string): string {
  if (!input) return input;
  return input.replace(SIGNAL_CITATION_RE, "").replace(/\s+/g, " ").trim();
}

/**
 * Resolve the per-deploy Signal detail URL pattern into a concrete URL
 * for a given UUID. Returns `null` when the env var is not configured,
 * in which case the UI should render the citation as unlinked
 * superscript. The pattern must contain the literal `<uuid>`
 * placeholder.
 */
export function resolveSignalUrl(uuid: string): string | null {
  const pattern = process.env.NEXT_PUBLIC_SIGNAL_DETAIL_URL_PATTERN;
  if (!pattern || !pattern.includes("<uuid>")) return null;
  return pattern.replace(/<uuid>/g, encodeURIComponent(uuid));
}
