import type { AddressData } from '@/lib/broadsheet/detail-types';
import { Icon } from '@/components/broadsheet/icons/Icon';

export function AddressA({ address }: { address: AddressData }) {
  return (
    <div className="address-a">
      <div className="address-a__street">
        <Icon name="place" size={14} className="address-a__icon" /> {address.street}
      </div>
      <div className="address-a__city-state mono-sm">
        {address.city}, {address.state} {address.zip}
      </div>
      {address.directionsUrl && (
        <a href={address.directionsUrl} className="address-a__directions mono-sm">
          Get Directions <Icon name="chevron-right" size={12} />
        </a>
      )}
    </div>
  );
}

export function AddressB({ address }: { address: AddressData }) {
  return (
    <div className="address-b">
      <div className="address-b__city">{address.city}, {address.state}</div>
      <span className="address-b__street">{address.street}</span>
      {' '}<span className="address-b__zip mono-sm">{address.zip}</span>
      {address.directionsUrl && (
        <>
          {' '}<a href={address.directionsUrl} className="address-b__map mono-sm">
            Map <Icon name="chevron-right" size={12} />
          </a>
        </>
      )}
    </div>
  );
}
