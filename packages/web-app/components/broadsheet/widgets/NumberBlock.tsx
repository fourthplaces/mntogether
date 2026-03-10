interface NumberBlockProps {
  number: string;
  label: string;
  detail?: string;
  color?: string;
}

export function NumberBlock({ number, label, detail, color = 'teal' }: NumberBlockProps) {
  const c = 'number-block';
  return (
    <div className={`${c} ${c}--${color}`} data-debug="Widget.numberBlock">
      <div className={`${c}__number condensed`}>{number}</div>
      <div className={`${c}__label condensed`}>{label}</div>
      {detail && <div className={`${c}__detail`}>{detail}</div>}
    </div>
  );
}
