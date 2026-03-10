interface SectionSepProps {
  title: string;
  sub?: string;
}

export function SectionSep({ title, sub }: SectionSepProps) {
  const c = 'section-sep';
  return (
    <div className={c} data-debug="Widget.sectionSep">
      <div className={`${c}__title`} dangerouslySetInnerHTML={{ __html: title }} />
      {sub && <div className={`${c}__sub`}>{sub}</div>}
    </div>
  );
}
