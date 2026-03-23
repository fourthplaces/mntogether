import { Icon } from '@/components/broadsheet/icons/Icon';

export interface PersonData {
  name: string;
  role?: string | null;
  bio?: string | null;
  photoUrl?: string | null;
  quote?: string | null;
}

/**
 * PersonCard A — Sidebar spotlight card.
 * Photo thumbnail, name (condensed), role (mono-sm), optional pull-quote (italic body).
 * Follows AddressA pattern: stacked vertical layout with left accent border.
 */
export function PersonCardA({ person }: { person: PersonData }) {
  return (
    <div className="person-card-a">
      <div className="person-card-a__header">
        {person.photoUrl ? (
          <img
            src={person.photoUrl}
            alt={person.name}
            className="person-card-a__photo"
          />
        ) : (
          <div className="person-card-a__photo-placeholder">
            <Icon name="person" size={20} />
          </div>
        )}
        <div className="person-card-a__info">
          <div className="person-card-a__name condensed">{person.name}</div>
          {person.role && (
            <div className="person-card-a__role mono-sm">{person.role}</div>
          )}
        </div>
      </div>
      {person.quote && (
        <blockquote className="person-card-a__quote">
          {person.quote}
        </blockquote>
      )}
      {person.bio && (
        <p className="person-card-a__bio">{person.bio}</p>
      )}
    </div>
  );
}

/**
 * PersonCard B — Compact byline card.
 * No photo, name + role inline. For tighter sidebar layouts.
 */
export function PersonCardB({ person }: { person: PersonData }) {
  return (
    <div className="person-card-b">
      <div className="person-card-b__name condensed">{person.name}</div>
      {person.role && (
        <span className="person-card-b__role mono-sm">{person.role}</span>
      )}
      {person.bio && (
        <p className="person-card-b__bio">{person.bio}</p>
      )}
    </div>
  );
}
