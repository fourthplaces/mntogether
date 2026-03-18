interface SectionSepProps {
  title: string;
  sub?: string;
  variant?: 'default' | 'ledger';
}

export function SectionSep({ title, sub, variant = 'default' }: SectionSepProps) {
  // Two visual treatments: default (.section-sep) and ledger (.led-section-break)
  // Both CSS classes exist in broadsheet.css. See DECISIONS_LOG.md.
  const c = variant === 'ledger' ? 'led-section-break' : 'section-sep';
  return (
    <div className={c} data-debug="Widget.sectionSep">
      <div className={`${c}__title`} dangerouslySetInnerHTML={{ __html: title }} />
      {sub && <div className={`${c}__sub`}>{sub}</div>}
    </div>
  );
}
