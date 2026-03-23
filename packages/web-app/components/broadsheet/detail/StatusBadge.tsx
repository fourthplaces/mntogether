import { Icon } from '@/components/broadsheet/icons/Icon';

/**
 * StatusBadge A — Exchange status indicator.
 *
 * Shows the state of an exchange post (available/needed) with optional
 * verification. Only renders if verified — unverified posts show nothing.
 *
 * state: "available" | "needed" — the exchange inventory state
 * verified: "verified" | "unverified" | date string — verification status
 */
export function StatusBadgeA({ state, verified }: {
  state: string;
  verified?: string | null;
}) {
  const isVerified = verified === 'verified' || (verified != null && verified !== 'unverified');
  if (!isVerified) return null;

  const label = state === 'available' ? 'Available' : state === 'needed' ? 'Needed' : state;

  return (
    <div className="status-badge-a">
      <Icon name="verified" size={12} className="status-badge-a__icon" />
      <span className="status-badge-a__label mono-sm">Verified</span>
      <span className="status-badge-a__sep"> &middot; </span>
      <span className="status-badge-a__state">{label}</span>
    </div>
  );
}

/**
 * StatusBadge B — Pill variant.
 */
export function StatusBadgeB({ state, verified }: {
  state: string;
  verified?: string | null;
}) {
  const isVerified = verified === 'verified' || (verified != null && verified !== 'unverified');
  if (!isVerified) return null;

  const label = state === 'available' ? 'Available' : state === 'needed' ? 'Needed' : state;

  return (
    <div className="status-badge-b">
      <span className="status-badge-b__pill status-badge-b__pill--verified mono-sm">
        <Icon name="verified" size={10} /> Verified
      </span>
      <span className="status-badge-b__pill mono-sm">{label}</span>
    </div>
  );
}
