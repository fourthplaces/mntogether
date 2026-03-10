/**
 * Molecule components — shared UI building blocks that post components compose.
 * Port of M.* functions from components.js as React components.
 * These produce the same HTML class structure as the prototype.
 */

import type { PostItem } from './types';

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
}

export function MTitle({ text, prefix, extra }: MTitleProps) {
  const className = `${prefix}__title${extra ? ' ' + extra : ''}`;
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
}

export function MKicker({ text, prefix, small }: MKickerProps) {
  return <div className={`${prefix}__kicker ${small ? 'mono-sm' : 'mono-md'}`} dangerouslySetInnerHTML={{ __html: text }} />;
}

// ── Tagline ─────────────────────────────────────
interface MTaglineProps {
  text: string;
  prefix: string;
}

export function MTagline({ text, prefix }: MTaglineProps) {
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
