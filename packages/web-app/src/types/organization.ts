// Organization and Business Info types

export interface Organization {
  id: string;
  name: string;
  description?: string;
  verified: boolean;
  contactInfo?: ContactInfo;
  location?: string;
  createdAt: string;
  updatedAt: string;
  businessInfo?: BusinessInfo;
  tags: Tag[];
}

export interface ContactInfo {
  email?: string;
  phone?: string;
  website?: string;
}

export interface BusinessInfo {
  proceedsPercentage?: number;
  proceedsBeneficiaryId?: string;
  donationLink?: string;
  giftCardLink?: string;
  onlineStoreUrl?: string;
  isCauseDriven: boolean;
}

export interface Tag {
  id: string;
  kind: string;
  value: string;
}

// Helper functions
export function getOwnershipTags(tags: Tag[]): Tag[] {
  return tags.filter(t => t.kind === 'ownership');
}

export function getCertificationTags(tags: Tag[]): Tag[] {
  return tags.filter(t => t.kind === 'certification');
}

export function getImpactAreaTags(tags: Tag[]): Tag[] {
  return tags.filter(t => t.kind === 'impact_area');
}

export function formatTagLabel(value: string): string {
  return value
    .split('_')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}
