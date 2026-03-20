import type { WebsiteData } from '@/lib/broadsheet/detail-types';
import { Icon } from '@/components/broadsheet/icons/Icon';

function extractDomain(url: string): string {
  try {
    return new URL(url).hostname.replace('www.', '');
  } catch {
    return url;
  }
}

export function WebsiteA({ website }: { website: WebsiteData }) {
  return (
    <div className="website-a">
      <Icon name="link" size={14} className="website-a__icon" />
      <a href={website.url} className="website-a__url mono-md">
        {website.label || extractDomain(website.url)}
      </a>
      <a href={website.url} className="website-a__cta mono-sm">
        Visit <Icon name="chevron-right" size={12} />
      </a>
    </div>
  );
}

export function WebsiteB({ website }: { website: WebsiteData }) {
  return (
    <div className="website-b">
      <div className="website-b__label mono-sm">{website.label || 'Website'}</div>
      <a href={website.url} className="website-b__url">
        {extractDomain(website.url)}
      </a>
      <div className="website-b__rule" />
    </div>
  );
}
