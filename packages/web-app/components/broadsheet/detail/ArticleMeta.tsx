interface ArticleMetaProps {
  parts: string[];
}

export function ArticleMeta({ parts }: ArticleMetaProps) {
  return (
    <div className="article-meta mono-md">
      {parts.map((part, i) => (
        <span key={i}>
          {part}
          {i < parts.length - 1 && <span className="sep">{'\u00b7'}</span>}
        </span>
      ))}
    </div>
  );
}
