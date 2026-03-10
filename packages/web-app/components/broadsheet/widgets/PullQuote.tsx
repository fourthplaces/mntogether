interface PullQuoteProps {
  quote: string;
  attribution: string;
}

export function PullQuote({ quote, attribution }: PullQuoteProps) {
  const c = 'pull-quote';
  return (
    <div className={c} data-debug="Widget.pullQuote">
      <div className={`${c}__text`}>&ldquo;{quote}&rdquo;</div>
      <div className={`${c}__attribution mono-md`}>{attribution}</div>
    </div>
  );
}
