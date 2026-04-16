/**
 * Molecule components — shared UI building blocks that post components compose.
 * Port of M.* functions from components.js as React components.
 * These produce the same HTML class structure as the prototype.
 */

import type { CSSProperties } from 'react';
import type { PostItem } from './types';

// ── Pencil mark wrapper ─────────────────────────
// Editorial emphasis overlay applied to a specific text element (title,
// kicker, deck, etc.). The CSS draws the SVG decoration via ::after, sized
// in em so it scales with the text. A random tilt is applied per render.
// Prototype tilt ranges (per style-guide.md):
//   star/heart: ±20°
//   smile:      0° to +30° (always leans right)
//   circle:     -2° to -10°
// The 4 semantic values stored in the database (one per post).
export type PencilMark = 'star' | 'heart' | 'smile' | 'circle';
// The 5 visual decoration styles (mark = flanking asterisks, no randomization).
export type PencilVariant = PencilMark | 'mark';

function randomTilt(variant: PencilVariant): string | undefined {
  switch (variant) {
    case 'mark':
      return undefined; // flanking asterisks are not tilted
    case 'smile':
      return `${(Math.random() * 30).toFixed(1)}deg`;
    case 'circle':
      return `${(Math.random() * -8 - 2).toFixed(1)}deg`;
    default: // star, heart
      return `${(Math.random() * 40 - 20).toFixed(1)}deg`;
  }
}

/**
 * Pencil component wraps a string with a pencil-{variant} span, applying a
 * random --tilt CSS variable used by the decoration pseudo-element to rotate.
 * Takes raw HTML string (matching the prototype's M.pencilStar(text) pattern).
 */
interface PencilProps {
  mark?: PencilVariant | null;
  text: string;
}

export function Pencil({ mark, text }: PencilProps) {
  if (!mark) return <span dangerouslySetInnerHTML={{ __html: text }} />;
  const tilt = randomTilt(mark);
  return (
    <span
      className={`pencil-${mark}`}
      style={tilt ? ({ '--tilt': tilt } as CSSProperties) : undefined}
      dangerouslySetInnerHTML={{ __html: text }}
    />
  );
}

// ── Tag ─────────────────────────────────────────
interface MTagProps {
  text: string;
  prefix: string;
  extra?: string;
}

export function MTag({ text, prefix, extra }: MTagProps) {
  const className = `${prefix}__tag post-tag mono-sm${extra ? ' ' + extra : ''}`;
  return <span className={className}>{text}</span>;
}

// ── Title ───────────────────────────────────────
interface MTitleProps {
  text: string;
  prefix: string;
  extra?: string;
  pencilMark?: PencilVariant | null;
}

export function MTitle({ text, prefix, extra, pencilMark }: MTitleProps) {
  const className = `${prefix}__title${extra ? ' ' + extra : ''}`;
  // 'circle' is only for date elements in event cards, never titles.
  const titleMark = pencilMark === 'circle' ? undefined : pencilMark;
  if (titleMark) {
    return (
      <div className={className}>
        <Pencil mark={titleMark} text={text} />
      </div>
    );
  }
  return <div className={className} dangerouslySetInnerHTML={{ __html: text }} />;
}

// ── Meta ────────────────────────────────────────
interface MMetaProps {
  text: string;
  prefix: string;
  small?: boolean;
}

export function MMeta({ text, prefix, small }: MMetaProps) {
  const className = `${prefix}__meta ${small ? 'mono-sm' : 'mono-md'}`;
  return <div className={className} dangerouslySetInnerHTML={{ __html: text }} />;
}

// ── Body ────────────────────────────────────────
interface MBodyProps {
  text: string;
  prefix: string;
  clamp?: number;
}

export function MBody({ text, prefix, clamp }: MBodyProps) {
  const className = `${prefix}__body${clamp ? ' clamp-' + clamp : ''}`;
  return <div className={className} dangerouslySetInnerHTML={{ __html: text }} />;
}

// ── RichBody ────────────────────────────────────
interface MRichBodyProps {
  paragraphs: string[];
  prefix: string;
  cols?: number;
  dropCap?: boolean;
}

export function MRichBody({ paragraphs, prefix, cols, dropCap }: MRichBodyProps) {
  const className = [prefix + '__body', cols === 2 ? 'col-flow-2' : ''].filter(Boolean).join(' ');
  return (
    <div className={className}>
      {paragraphs.map((p, i) => (
        <p
          key={i}
          className={i === 0 && dropCap ? 'drop-cap' : undefined}
          dangerouslySetInnerHTML={{ __html: p }}
        />
      ))}
    </div>
  );
}

// ── ReadMore ────────────────────────────────────
interface MReadMoreProps {
  href?: string;
  text?: string;
}

export function MReadMore({ href, text }: MReadMoreProps) {
  return (
    <span className="read-more mono-sm">
      {text || 'Read more'} &rarr;
    </span>
  );
}

// ── Contact ─────────────────────────────────────
interface MContactProps {
  text: string;
  prefix: string;
}

export function MContact({ text, prefix }: MContactProps) {
  return <div className={`${prefix}__contact mono-sm`} dangerouslySetInnerHTML={{ __html: text }} />;
}

// ── Status ──────────────────────────────────────
interface MStatusProps {
  text: string;
  prefix: string;
  extra?: string;
}

export function MStatus({ text, prefix, extra }: MStatusProps) {
  return <span className={`${prefix}__status${extra ? ' ' + extra : ' mono-sm'}`}>{text}</span>;
}

// ── CtaLink ─────────────────────────────────────
interface MCtaLinkProps {
  href?: string;
  text: string;
  prefix: string;
  small?: boolean;
}

export function MCtaLink({ href, text, prefix, small }: MCtaLinkProps) {
  return (
    <a href={href || '#'} className={`${prefix}__link ${small ? 'mono-sm' : 'mono-md'}`}>
      {text} &rarr;
    </a>
  );
}

// ── Time ────────────────────────────────────────
interface MTimeProps {
  text: string;
  prefix: string;
}

export function MTime({ text, prefix }: MTimeProps) {
  return <span className={`${prefix}__time mono-sm`}>{text}</span>;
}

// ── Kicker ──────────────────────────────────────
interface MKickerProps {
  text: string;
  prefix: string;
  small?: boolean;
  pencilMark?: PencilVariant | null;
}

export function MKicker({ text, prefix, small, pencilMark }: MKickerProps) {
  const className = `${prefix}__kicker ${small ? 'mono-sm' : 'mono-md'}`;
  if (pencilMark) {
    return (
      <div className={className}>
        <Pencil mark={pencilMark} text={text} />
      </div>
    );
  }
  return <div className={className} dangerouslySetInnerHTML={{ __html: text }} />;
}

// ── Tagline ─────────────────────────────────────
interface MTaglineProps {
  text: string;
  prefix: string;
  pencilMark?: PencilVariant | null;
}

export function MTagline({ text, prefix, pencilMark }: MTaglineProps) {
  if (pencilMark) {
    return (
      <div className={`${prefix}__tagline`}>
        <Pencil mark={pencilMark} text={text} />
      </div>
    );
  }
  return <div className={`${prefix}__tagline`} dangerouslySetInnerHTML={{ __html: text }} />;
}

// ── When ────────────────────────────────────────
interface MWhenProps {
  text: string;
  prefix: string;
  md?: boolean;
}

export function MWhen({ text, prefix, md }: MWhenProps) {
  return <div className={`${prefix}__when ${md ? 'mono-md' : 'mono-sm'}`}>{text}</div>;
}

// ── Updated ─────────────────────────────────────
interface MUpdatedProps {
  text: string;
  prefix: string;
}

export function MUpdated({ text, prefix }: MUpdatedProps) {
  return <div className={`${prefix}__updated mono-sm`}>{text}</div>;
}

// ── ResourceList ────────────────────────────────
interface MResourceListProps {
  items: PostItem[];
  prefix: string;
  cols?: number;
}

export function MResourceList({ items, prefix, cols }: MResourceListProps) {
  const list = (
    <ul className={`${prefix}__list`}>
      {items.map((item, i) => (
        <li key={i}>
          <strong>{item.name}</strong> &mdash; {item.detail}
        </li>
      ))}
    </ul>
  );
  return cols === 2 ? <div className="col-flow-2">{list}</div> : list;
}
