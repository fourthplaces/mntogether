import type { LinkData } from '@/lib/broadsheet/detail-types';
import { Icon } from '@/components/broadsheet/icons/Icon';

function extractDomain(url: string): string {
  try {
    return new URL(url).hostname.replace('www.', '');
  } catch {
    return '';
  }
}

interface LinksAProps {
  links: LinkData[];
  header?: string;
}

export function LinksA({ links, header = 'See Also' }: LinksAProps) {
  return (
    <div className="links-a">
      {header && <div className="links-a__header mono-sm">{header}</div>}
      {links.map((link, i) => {
        const domain = extractDomain(link.url);
        return (
          <a key={i} href={link.url || '#'} className="links-a__item">
            <span className="links-a__arrow"><Icon name="chevron-right" size={14} /></span>
            <span className="links-a__title">{link.title}</span>
            {domain && <span className="links-a__domain mono-sm">{domain}</span>}
          </a>
        );
      })}
    </div>
  );
}

export function LinksB({ links }: { links: LinkData[] }) {
  return (
    <div className="links-b">
      {links.map((link, i) => {
        const domain = extractDomain(link.url);
        return (
          <a key={i} href={link.url || '#'} className="links-b__item">
            <span className="links-b__num">{i + 1}</span>
            <div>
              <span className="links-b__title">{link.title}</span>
              {domain && <span className="links-b__domain mono-sm">{domain}</span>}
            </div>
          </a>
        );
      })}
    </div>
  );
}
