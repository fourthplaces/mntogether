import { Icon } from '@/components/broadsheet/icons/Icon';

export function ArticleNav() {
  return (
    <nav className="article-nav">
      <a href="/" className="article-nav__back">
        <Icon name="arrow-back" size={14} /> Back to front page
      </a>
      <a href="/" className="article-nav__masthead">Minnesota, Together.</a>
    </nav>
  );
}
