import type { EmailData } from '@/lib/broadsheet/detail-types';
import { Icon } from '@/components/broadsheet/icons/Icon';

export function EmailA({ email }: { email: EmailData }) {
  return (
    <div className="email-a">
      <Icon name="email" size={14} className="email-a__icon" />
      <a href={`mailto:${email.address}`} className="email-a__address mono-md">
        {email.address}
      </a>
      <a href={`mailto:${email.address}`} className="email-a__cta mono-sm">
        Email <Icon name="chevron-right" size={12} />
      </a>
    </div>
  );
}

export function EmailB({ email }: { email: EmailData }) {
  return (
    <div className="email-b">
      <div className="email-b__label mono-sm">{email.label || 'Email'}</div>
      <a href={`mailto:${email.address}`} className="email-b__address">
        {email.address}
      </a>
      <div className="email-b__rule" />
    </div>
  );
}
