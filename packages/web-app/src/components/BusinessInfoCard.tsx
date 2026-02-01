import { BusinessInfo, Tag, formatTagLabel } from '../types/organization';

interface BusinessInfoCardProps {
  businessInfo?: BusinessInfo;
  tags: Tag[];
  organizationName: string;
}

export function BusinessInfoCard({ businessInfo, tags }: BusinessInfoCardProps) {
  if (!businessInfo) {
    return null;
  }

  const ownershipTags = tags.filter(t => t.kind === 'ownership');
  const certificationTags = tags.filter(t => t.kind === 'certification');
  const impactAreas = tags.filter(t => t.kind === 'impact_area');

  const getBadgeEmoji = (value: string): string => {
    const emojiMap: Record<string, string> = {
      women_owned: '👩',
      minority_owned: '🌍',
      lgbtq_owned: '🏳️‍🌈',
      veteran_owned: '🎖️',
      immigrant_owned: '✈️',
      bipoc_owned: '✊',
      b_corp: '🏆',
      benefit_corp: '💼',
      worker_owned: '🤝',
      cooperative: '🤝',
    };
    return emojiMap[value] || '•';
  };

  return (
    <div className="bg-gradient-to-br from-amber-50 to-orange-50 rounded-lg border border-amber-200 p-6 space-y-4">
      {/* Cause-Driven Badge */}
      {businessInfo.isCauseDriven && businessInfo.proceedsPercentage && (
        <div className="inline-flex items-center gap-2 bg-green-100 text-green-800 px-4 py-2 rounded-full border border-green-300">
          <span className="text-lg">🤝</span>
          <span className="font-semibold">
            {businessInfo.proceedsPercentage}% goes to charity
          </span>
        </div>
      )}

      {/* Ownership & Certification Badges */}
      {(ownershipTags.length > 0 || certificationTags.length > 0) && (
        <div className="flex flex-wrap gap-2">
          {ownershipTags.map((tag) => (
            <span
              key={tag.id}
              className="inline-flex items-center gap-1 bg-purple-100 text-purple-800 px-3 py-1 rounded-full text-sm border border-purple-300"
            >
              <span>{getBadgeEmoji(tag.value)}</span>
              <span>{formatTagLabel(tag.value)}</span>
            </span>
          ))}
          {certificationTags.map((tag) => (
            <span
              key={tag.id}
              className="inline-flex items-center gap-1 bg-blue-100 text-blue-800 px-3 py-1 rounded-full text-sm border border-blue-300"
            >
              <span>{getBadgeEmoji(tag.value)}</span>
              <span>{formatTagLabel(tag.value)}</span>
            </span>
          ))}
        </div>
      )}

      {/* Impact Areas */}
      {impactAreas.length > 0 && (
        <div>
          <h4 className="text-sm font-semibold text-stone-700 mb-2">Impact Areas:</h4>
          <div className="flex flex-wrap gap-2">
            {impactAreas.map((tag) => (
              <span
                key={tag.id}
                className="bg-white text-stone-700 px-3 py-1 rounded-md text-sm border border-stone-300"
              >
                {formatTagLabel(tag.value)}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Call-to-Action Links */}
      <div className="flex flex-wrap gap-3 pt-2">
        {businessInfo.onlineStoreUrl && (
          <a
            href={businessInfo.onlineStoreUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 bg-amber-700 text-white px-4 py-2 rounded-md hover:bg-amber-800 transition-colors"
          >
            <span>🛍️</span>
            <span>Shop & Support</span>
          </a>
        )}
        {businessInfo.donationLink && (
          <a
            href={businessInfo.donationLink}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 bg-green-600 text-white px-4 py-2 rounded-md hover:bg-green-700 transition-colors"
          >
            <span>❤️</span>
            <span>Donate</span>
          </a>
        )}
        {businessInfo.giftCardLink && (
          <a
            href={businessInfo.giftCardLink}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 bg-blue-600 text-white px-4 py-2 rounded-md hover:bg-blue-700 transition-colors"
          >
            <span>🎁</span>
            <span>Buy Gift Card</span>
          </a>
        )}
      </div>
    </div>
  );
}
