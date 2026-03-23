type IconName = 'chevron-right' | 'arrow-back' | 'play-arrow' | 'place' | 'phone' | 'open-in-new' | 'email' | 'link' | 'person' | 'verified';

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
}

export function Icon({ name, size = 18, className }: IconProps) {
  return (
    <svg
      className={`icon ${className ?? ''}`}
      width={size}
      height={size}
      fill="currentColor"
      aria-hidden="true"
    >
      <use href={`/icons.svg#icon-${name}`} />
    </svg>
  );
}
