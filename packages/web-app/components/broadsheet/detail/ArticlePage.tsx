import type { ReactNode } from 'react';

interface ArticlePageProps {
  main: ReactNode;
  sidebar: ReactNode;
}

export function ArticlePage({ main, sidebar }: ArticlePageProps) {
  return (
    <div className="article-page">
      <div className="article-main">{main}</div>
      <aside className="article-sidebar">{sidebar}</aside>
    </div>
  );
}
