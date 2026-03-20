import Link from 'next/link';
import type { RelatedArticle } from '@/lib/broadsheet/detail-types';
import { Icon } from '@/components/broadsheet/icons/Icon';

export function RelatedA({ articles }: { articles: RelatedArticle[] }) {
  if (articles.length === 0) return null;
  return (
    <div className="related-a">
      <div className="related-a__header condensed">More from Minnesota, Together</div>
      <div className="related-a__grid">
        {articles.map((a) => (
          <Link key={a.id} href={`/posts/${a.id}`} className="related-a__card">
            <div className="related-a__kicker mono-sm">{a.kicker}</div>
            <div className="related-a__title">{a.title}</div>
            {a.meta && <div className="related-a__meta mono-sm">{a.meta}</div>}
          </Link>
        ))}
      </div>
    </div>
  );
}

export function RelatedB({ articles }: { articles: RelatedArticle[] }) {
  if (articles.length === 0) return null;
  return (
    <div className="related-b">
      <div className="related-b__header mono-sm">Related</div>
      {articles.map((a) => (
        <Link key={a.id} href={`/posts/${a.id}`} className="related-b__item">
          <span className="related-b__section mono-sm" style={{ background: a.color || 'var(--deep-forest)' }}>
            {a.kicker}
          </span>
          <span className="related-b__title">{a.title}</span>
          <span className="related-b__arrow"><Icon name="chevron-right" size={14} /></span>
        </Link>
      ))}
    </div>
  );
}
