/**
 * Block catalog — registry of all insertable block types for the editor.
 *
 * Used by the slash command menu and the + block insert handle.
 * Each entry defines the Plate node type, display info, and default data.
 */

export interface BlockCatalogEntry {
  key: string;
  label: string;
  description: string;
  category: "text" | "media" | "editorial" | "data";
  /** Default node data (merged into the TElement on insert) */
  defaultData?: Record<string, unknown>;
}

export const BLOCK_CATALOG: BlockCatalogEntry[] = [
  // ── Text ──
  { key: "p", label: "Paragraph", description: "Plain text block", category: "text" },
  { key: "h2", label: "Heading 2", description: "Large section heading", category: "text" },
  { key: "h3", label: "Heading 3", description: "Medium section heading", category: "text" },
  { key: "h4", label: "Heading 4", description: "Small section heading", category: "text" },
  { key: "blockquote", label: "Blockquote", description: "Indented quote with left border", category: "text" },

  // ── Media ──
  { key: "photo_a", label: "Photo (Full Width)", description: "Contained image with caption", category: "media", defaultData: { src: "", caption: "", credit: "" } },
  { key: "photo_b", label: "Photo (Bleed)", description: "Full-bleed image spanning columns", category: "media", defaultData: { src: "", caption: "", credit: "" } },
  { key: "photo_block", label: "Photo (Inline)", description: "Floating image in text flow", category: "media", defaultData: { src: "", caption: "", credit: "", variant: "c" } },
  { key: "audio_a", label: "Audio (Waveform)", description: "Waveform strip player", category: "media", defaultData: { title: "", duration: "", credit: "" } },
  { key: "audio_b", label: "Audio (Transcript)", description: "Transcript card with excerpt", category: "media", defaultData: { title: "", duration: "", excerpt: "" } },

  // ── Editorial ──
  { key: "pull_quote", label: "Pull Quote", description: "Highlighted editorial quote", category: "editorial" },
  { key: "section_break", label: "Section Break", description: "Decorative divider (· · ·)", category: "editorial" },
  { key: "kicker_a", label: "Kicker (Tags)", description: "Middot-separated section tags", category: "editorial", defaultData: { tags: [""] } },
  { key: "kicker_b", label: "Kicker (Folio)", description: "Primary tag with colored border", category: "editorial", defaultData: { primary: "", secondary: [], color: "" } },
  { key: "article_meta", label: "Article Meta", description: "Byline · Date · Location bar", category: "editorial", defaultData: { parts: [""] } },

  // ── Data ──
  { key: "links_box", label: "Links (See Also)", description: "Reference box with titled links", category: "data", defaultData: { header: "See Also", links: [{ title: "", url: "" }] } },
  { key: "links_b", label: "Links (Margin)", description: "Numbered margin references", category: "data", defaultData: { links: [{ title: "", url: "" }] } },
  { key: "list_a", label: "List (Editorial)", description: "Bullet editorial list", category: "data", defaultData: { items: [""], ordered: false } },
  { key: "list_b", label: "List (Ledger)", description: "Ledger ruled list", category: "data", defaultData: { items: [""], ordered: false } },
  { key: "resource_list", label: "Resource List", description: "Name · detail pairs", category: "data", defaultData: { items: [{ name: "", detail: "" }] } },
  { key: "resource_list_b", label: "Resource List (Ledger)", description: "Name · detail with ruled lines", category: "data", defaultData: { items: [{ name: "", detail: "" }] } },
  { key: "address_a", label: "Address (Block)", description: "Ledger address with directions", category: "data", defaultData: { street: "", city: "", state: "", zip: "", directionsUrl: "" } },
  { key: "address_b", label: "Address (Dateline)", description: "Inline newspaper-style address", category: "data", defaultData: { street: "", city: "", state: "", zip: "" } },
  { key: "phone_a", label: "Phone (Classified)", description: "Contact strip with call CTA", category: "data", defaultData: { number: "", display: "", label: "" } },
  { key: "phone_b", label: "Phone (Display)", description: "Masthead-style phone number", category: "data", defaultData: { number: "", display: "", label: "" } },
];

export const CATEGORY_LABELS: Record<string, string> = {
  text: "Text",
  media: "Media",
  editorial: "Editorial",
  data: "Data & Info",
};
