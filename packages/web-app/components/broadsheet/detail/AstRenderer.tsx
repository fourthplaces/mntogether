/**
 * AstRenderer — renders Plate.js JSON AST (Value) to React elements
 * with broadsheet CSS classes.
 *
 * This is a lightweight read-only renderer — no Plate.js dependency needed.
 * It maps each node type to the appropriate broadsheet detail component or
 * CSS class.
 *
 * Custom node types:
 * - pull_quote → .pull-quote (floated right on web-app)
 * - section_break → .section-break (centered decorative dots)
 * - photo_block → PhotoC / PhotoD component
 * - links_box → LinksA component
 * - resource_list → .list-a--resource
 */

import React from "react";
import { PhotoC, PhotoD } from "./Photo";
import { LinksA } from "./Links";

// ---------------------------------------------------------------------------
// Types — minimal subset of Plate.js AST
// ---------------------------------------------------------------------------

interface AstText {
  text: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
}

interface AstElement {
  type?: string;
  children?: AstNode[];
  // Photo block attrs
  src?: string;
  caption?: string;
  credit?: string;
  variant?: "c" | "d";
  // Links box attrs
  header?: string;
  links?: Array<{ title: string; url: string }>;
  // Resource list attrs
  items?: Array<{ name: string; detail: string }>;
  // Link attrs
  url?: string;
}

type AstNode = AstText | AstElement;

function isText(node: AstNode): node is AstText {
  return "text" in node && typeof (node as AstText).text === "string";
}

// ---------------------------------------------------------------------------
// Text renderer — handles marks (bold, italic, underline)
// ---------------------------------------------------------------------------

function renderText(node: AstText, key: number): React.ReactNode {
  let content: React.ReactNode = node.text;
  if (!content) return null;

  if (node.bold) content = <strong key={`b-${key}`}>{content}</strong>;
  if (node.italic) content = <em key={`i-${key}`}>{content}</em>;
  if (node.underline) content = <u key={`u-${key}`}>{content}</u>;

  return <React.Fragment key={key}>{content}</React.Fragment>;
}

// ---------------------------------------------------------------------------
// Element renderer — maps node type to broadsheet elements
// ---------------------------------------------------------------------------

function renderChildren(children?: AstNode[]): React.ReactNode {
  if (!children) return null;
  return children.map((child, i) => {
    if (isText(child)) return renderText(child, i);
    return renderElement(child, i);
  });
}

function renderElement(node: AstElement, key: number): React.ReactNode {
  const children = renderChildren(node.children);

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
    case "blockquote":
      return <blockquote key={key}>{children}</blockquote>;
    case "ul":
      return <ul key={key}>{children}</ul>;
    case "ol":
      return <ol key={key}>{children}</ol>;
    case "li":
      return <li key={key}>{children}</li>;
    case "a":
      return <a key={key} href={node.url || "#"}>{children}</a>;

    // Custom blocks
    case "pull_quote":
      return (
        <blockquote key={key} className="pull-quote">
          {children}
        </blockquote>
      );

    case "section_break":
      return (
        <div key={key} className="section-break">
          · · ·
        </div>
      );

    case "photo_block": {
      const photo = {
        src: node.src || "",
        alt: node.caption || "",
        caption: node.caption || "",
        credit: node.credit || "",
      };
      if (node.variant === "d") {
        return <PhotoD key={key} photo={photo} />;
      }
      return <PhotoC key={key} photo={photo} />;
    }

    case "links_box":
      return (
        <LinksA
          key={key}
          links={node.links || []}
          header={node.header || "See Also"}
        />
      );

    case "resource_list":
      return (
        <ul key={key} className="list-a list-a--resource">
          {(node.items || []).map((item, i) => (
            <li key={i}>
              <strong>{item.name}</strong>
              {item.detail && <> · {item.detail}</>}
            </li>
          ))}
        </ul>
      );

    // Default: render children in a div
    default:
      return <div key={key}>{children}</div>;
  }
}

// ---------------------------------------------------------------------------
// Public component
// ---------------------------------------------------------------------------

interface AstRendererProps {
  /** Plate.js Value — array of top-level AST nodes */
  value: AstNode[];
  /** CSS class for the wrapper (default: "body-a") */
  className?: string;
}

export function AstRenderer({ value, className = "body-a" }: AstRendererProps) {
  return (
    <div className={className}>
      {value.map((node, i) => {
        if (isText(node)) return renderText(node, i);
        return renderElement(node, i);
      })}
    </div>
  );
}
