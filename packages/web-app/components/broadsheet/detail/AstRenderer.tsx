/**
 * AstRenderer — renders Plate.js JSON AST (Value) to React elements
 * with broadsheet CSS classes.
 *
 * Lightweight read-only renderer — no Plate.js dependency needed.
 * Maps each node type to the appropriate broadsheet detail component.
 */

import React from "react";
import { PhotoA, PhotoB, PhotoC, PhotoD } from "./Photo";
import { LinksA, LinksB } from "./Links";
import { ListA, ListB, ResourceListA, ResourceListB } from "./List";
import { AddressA, AddressB } from "./Address";
import { PhoneA, PhoneB } from "./Phone";
import { KickerA, KickerB } from "./Kicker";
import { ArticleMeta } from "./ArticleMeta";
import { AudioA, AudioB } from "./Audio";
import { CitationText } from "./Citation";
import { CitationIndex } from "@/lib/broadsheet/citations";

// ---------------------------------------------------------------------------
// Types — minimal subset of Plate.js AST
// ---------------------------------------------------------------------------

interface AstText {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
  code?: boolean;
}

interface AstElement {
  type?: string;
  children?: AstNode[];
  // Common data attrs (varies by node type)
  src?: string;
  caption?: string;
  credit?: string;
  variant?: "c" | "d";
  header?: string;
  links?: Array<{ title: string; url: string }>;
  items?: Array<{ name: string; detail: string }> | string[];
  url?: string;
  tags?: string[];
  primary?: string;
  secondary?: string[];
  color?: string;
  parts?: string[];
  ordered?: boolean;
  title?: string;
  duration?: string;
  excerpt?: string;
  street?: string;
  city?: string;
  state?: string;
  zip?: string;
  directionsUrl?: string;
  number?: string;
  display?: string;
  label?: string;
  // Todo / toggle / callout / code_block
  checked?: boolean;
  open?: boolean;
  emoji?: string;
  language?: string;
}

type AstNode = AstText | AstElement;

function isText(node: AstNode): node is AstText {
  return "text" in node && typeof (node as AstText).text === "string";
}

// ---------------------------------------------------------------------------
// Text renderer — handles marks
// ---------------------------------------------------------------------------

function renderText(node: AstText, key: number, citationIndex?: CitationIndex): React.ReactNode {
  if (!node.text) return null;

  // Parse inline [signal:UUID] tokens before applying marks so the
  // superscript cites are nested inside any bold / italic / etc.
  let content: React.ReactNode =
    citationIndex && node.text.includes("[signal:")
      ? <CitationText text={node.text} index={citationIndex} />
      : node.text;

  if (node.bold) content = <strong key={`b-${key}`}>{content}</strong>;
  if (node.italic) content = <em key={`i-${key}`}>{content}</em>;
  if (node.underline) content = <u key={`u-${key}`}>{content}</u>;
  if (node.strikethrough) content = <s key={`s-${key}`}>{content}</s>;
  if (node.code) content = <code key={`c-${key}`}>{content}</code>;

  return <React.Fragment key={key}>{content}</React.Fragment>;
}

// ---------------------------------------------------------------------------
// Element renderer
// ---------------------------------------------------------------------------

function renderChildren(children: AstNode[] | undefined, citationIndex?: CitationIndex): React.ReactNode {
  if (!children) return null;
  return children.map((child, i) => {
    if (isText(child)) return renderText(child, i, citationIndex);
    return renderElement(child, i, citationIndex);
  });
}

function renderElement(node: AstElement, key: number, citationIndex?: CitationIndex): React.ReactNode {
  const children = renderChildren(node.children, citationIndex);

  switch (node.type) {
    // Standard block elements
    case "p":
      return <p key={key}>{children}</p>;
    case "h2":
      return <h2 key={key}>{children}</h2>;
    case "h3":
      return <h3 key={key}>{children}</h3>;
    case "h4":
      return <h4 key={key}>{children}</h4>;
    case "h5":
      return <h5 key={key}>{children}</h5>;
    case "h6":
      return <h6 key={key}>{children}</h6>;
    case "blockquote":
      return <blockquote key={key}>{children}</blockquote>;
    case "ul":
      return <ul key={key} className="list-a">{children}</ul>;
    case "ol":
      return <ol key={key} className="list-a">{children}</ol>;
    case "li":
      return <li key={key}>{children}</li>;
    case "a":
      return <a key={key} href={node.url || "#"}>{children}</a>;

    // Notion-style blocks
    case "todo":
      return (
        <div key={key} className="todo-item">
          <input type="checkbox" checked={!!node.checked} readOnly className="todo-checkbox" />
          <span className={node.checked ? "todo-text todo-text--checked" : "todo-text"}>{children}</span>
        </div>
      );
    case "toggle":
      return (
        <details key={key} open={node.open !== false}>
          <summary className="toggle-summary">{children}</summary>
        </details>
      );
    case "callout":
      return (
        <div key={key} className="callout">
          {node.emoji && <span className="callout-emoji">{node.emoji}</span>}
          <div className="callout-content">{children}</div>
        </div>
      );
    case "code_block":
      return (
        <pre key={key} className="code-block">
          <code>{children}</code>
        </pre>
      );

    // Pull quote (floated right on web-app)
    case "pull_quote":
      return (
        <blockquote key={key} className="pull-quote">
          {children}
        </blockquote>
      );

    // Section break
    case "section_break":
      return <div key={key} className="section-break">· · ·</div>;

    // Photos
    case "photo_a":
      return <PhotoA key={key} photo={{ src: node.src || "", alt: node.caption || "", caption: node.caption || "", credit: node.credit || "" }} />;
    case "photo_b":
      return <PhotoB key={key} photo={{ src: node.src || "", alt: node.caption || "", caption: node.caption || "", credit: node.credit || "" }} />;
    case "photo_block": {
      const photo = { src: node.src || "", alt: node.caption || "", caption: node.caption || "", credit: node.credit || "" };
      return node.variant === "d" ? <PhotoD key={key} photo={photo} /> : <PhotoC key={key} photo={photo} />;
    }

    // Audio
    case "audio_a":
      return <AudioA key={key} audio={{ title: node.title || "", duration: node.duration || "", credit: node.credit }} />;
    case "audio_b":
      return <AudioB key={key} audio={{ title: node.title || "", duration: node.duration || "", excerpt: node.excerpt }} />;

    // Kickers
    case "kicker_a":
      return <KickerA key={key} tags={node.tags || []} />;
    case "kicker_b":
      return <KickerB key={key} primary={node.primary || ""} secondary={node.secondary} color={node.color} />;

    // Article meta
    case "article_meta":
      return <ArticleMeta key={key} parts={node.parts || []} />;

    // Links
    case "links_box":
      return <LinksA key={key} links={node.links || []} header={node.header || "See Also"} />;
    case "links_b":
      return <LinksB key={key} links={node.links || []} />;

    // Lists
    case "list_a": {
      const stringItems = (node.items || []) as string[];
      return node.ordered
        ? <ListA key={key} items={stringItems} ordered />
        : <ListA key={key} items={stringItems} />;
    }
    case "list_b": {
      const stringItems = (node.items || []) as string[];
      return node.ordered
        ? <ListB key={key} items={stringItems} ordered />
        : <ListB key={key} items={stringItems} />;
    }

    // Resource lists
    case "resource_list": {
      const resItems = (node.items || []) as Array<{ name: string; detail: string }>;
      return <ResourceListA key={key} items={resItems} />;
    }
    case "resource_list_b": {
      const resItems = (node.items || []) as Array<{ name: string; detail: string }>;
      return <ResourceListB key={key} items={resItems} />;
    }

    // Addresses
    case "address_a":
      return <AddressA key={key} address={{ street: node.street || "", city: node.city || "", state: node.state || "", zip: node.zip || "", directionsUrl: node.directionsUrl }} />;
    case "address_b":
      return <AddressB key={key} address={{ street: node.street || "", city: node.city || "", state: node.state || "", zip: node.zip || "" }} />;

    // Phones
    case "phone_a":
      return <PhoneA key={key} phone={{ number: node.number || "", display: node.display, label: node.label }} />;
    case "phone_b":
      return <PhoneB key={key} phone={{ number: node.number || "", display: node.display, label: node.label }} />;

    // Default: render children in a div
    default:
      return <div key={key}>{children}</div>;
  }
}

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

interface AstRendererProps {
  value: AstNode[];
  className?: string;
  /**
   * Shared citation registry for `[signal:UUID]` tokens in text
   * nodes. Pass a single instance spanning the whole body render so
   * numbering is stable. When omitted, citations render as literal
   * text.
   */
  citationIndex?: CitationIndex;
}

export function AstRenderer({ value, className = "body-a", citationIndex }: AstRendererProps) {
  return (
    <div className={className}>
      {value.map((node, i) => {
        if (isText(node)) return renderText(node, i, citationIndex);
        return renderElement(node, i, citationIndex);
      })}
    </div>
  );
}
