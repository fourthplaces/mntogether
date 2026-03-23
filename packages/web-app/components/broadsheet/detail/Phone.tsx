import type { PhoneData } from '@/lib/broadsheet/detail-types';
import { Icon } from '@/components/broadsheet/icons/Icon';

export function PhoneA({ phone }: { phone: PhoneData }) {
  return (
    <div className="phone-a">
      <Icon name="phone" size={14} className="phone-a__icon" />
      <a href={`tel:${phone.number}`} className="phone-a__number mono-md">
        {phone.display || phone.number}
      </a>
    </div>
  );
}

export function PhoneB({ phone }: { phone: PhoneData }) {
  return (
    <div className="phone-b">
      <div className="phone-b__label mono-sm">{phone.label || 'Telephone'}</div>
      <a href={`tel:${phone.number}`} className="phone-b__number">
        {phone.display || phone.number}
      </a>
      <div className="phone-b__rule" />
    </div>
  );
}
